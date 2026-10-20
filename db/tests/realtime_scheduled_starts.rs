mod common;

use chrono::{DateTime, Duration, Utc};
use common::{
    db::test_db,
    tournament::{
        create_rr_tournament_with_configuration,
        create_user,
        realtime_clock,
        round_robin_configuration,
    },
};
use db_lib::{get_conn, schema::tournaments, tournaments::fixed_field, DbConn};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::{
    tournament::{
        elimination::{
            ClinchPolicy,
            Config as EliminationConfig,
            EntrantSide,
            SeriesPhase,
            SeriesPlan,
            SetLimit,
            Topology,
        },
        round_robin::Config as RoundRobinConfig,
        BotAdmission,
        Clock,
        Config,
        CorrespondenceClock,
        FormatConfig,
    },
    TournamentStatus,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn correspondence_round_robin() -> Config {
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::RoundRobin(RoundRobinConfig::standard(
            NonZeroU32::new(1).unwrap(),
            Clock::Correspondence(CorrespondenceClock::DaysPerMove {
                seconds_per_move: NonZeroU32::new(86_400).unwrap(),
            }),
        )),
    }
}

fn realtime_elimination() -> Config {
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Elimination(EliminationConfig {
            topology: Topology::Single { bronze: false },
            default_plan: SeriesPlan {
                phases: vec![SeriesPhase {
                    games_per_set: 1,
                    color_order: vec![EntrantSide::First],
                    set_limit: SetLimit::UntilDecisive,
                    clinch: ClinchPolicy::PlayAll,
                    clock: realtime_clock(),
                }],
            },
            stage_overrides: Vec::new(),
        }),
    }
}

async fn due_tournament(
    organizer_id: Uuid,
    name: &str,
    configuration: Config,
    due: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Uuid {
    let tournament = create_rr_tournament_with_configuration(
        organizer_id,
        name,
        TournamentStatus::NotStarted,
        Some(Utc::now() + Duration::hours(1)),
        configuration,
        conn,
    )
    .await;
    diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::starts_at.eq(Some(due)))
        .execute(conn)
        .await
        .expect("make tournament due");
    tournament.id
}

#[tokio::test(flavor = "multi_thread")]
async fn due_realtime_tournaments_do_not_crowd_out_correspondence_starts() {
    let db = test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("test database connection");
    let organizer = create_user("paused_organizer", &mut conn).await;
    let as_of = Utc::now();
    let realtime_round_robin = due_tournament(
        organizer.id,
        "Realtime round robin",
        round_robin_configuration(),
        as_of - Duration::minutes(3),
        &mut conn,
    )
    .await;
    let realtime_elimination = due_tournament(
        organizer.id,
        "Realtime elimination",
        realtime_elimination(),
        as_of - Duration::minutes(2),
        &mut conn,
    )
    .await;
    let correspondence = due_tournament(
        organizer.id,
        "Correspondence round robin",
        correspondence_round_robin(),
        as_of - Duration::minutes(1),
        &mut conn,
    )
    .await;

    assert_eq!(
        fixed_field::scheduled_start_candidates(as_of, 2, &mut conn)
            .await
            .expect("load every due tournament"),
        vec![realtime_round_robin, realtime_elimination],
    );
    assert_eq!(
        fixed_field::scheduled_non_realtime_start_candidates(as_of, 2, &mut conn)
            .await
            .expect("load due tournaments that are not realtime"),
        vec![correspondence],
    );
}
