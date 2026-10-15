mod common;

use chrono::{Duration as ChronoDuration, Utc};
use common::tournament::{create_rr_tournament_with_configuration, create_user};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Tournament, TournamentUser},
    schema::tournaments,
    tournaments::arena,
};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use shared_types::{
    tournament::{arena::Config as ArenaConfig, BotAdmission, Config, FormatConfig, RealtimeClock},
    TournamentId,
    TournamentStatus,
};
use std::{collections::HashSet, num::NonZeroU32, time::Duration};
use tokio::{
    task::JoinSet,
    time::{sleep, timeout},
};

#[derive(QueryableByName)]
struct Backend {
    #[diesel(sql_type = Integer)]
    pid: i32,
}

#[derive(QueryableByName)]
struct BlockedJoins {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_arena_joins_survive_a_featured_game_change() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get setup connection");
    let organizer = create_user("arena_join_org", &mut conn).await;
    let mut players = Vec::new();
    for index in 0..8 {
        players.push(
            create_user(&format!("arena_join_{index}"), &mut conn)
                .await
                .id,
        );
    }
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "Concurrent arena joins",
        TournamentStatus::NotStarted,
        Some(Utc::now() + ChronoDuration::hours(1)),
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig::new(
                NonZeroU32::new(3_600).unwrap(),
                RealtimeClock {
                    base_seconds: NonZeroU32::new(180).unwrap(),
                    increment_seconds: 1,
                },
            )),
        },
        &mut conn,
    )
    .await;
    diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::starts_at.eq(Some(Utc::now() - ChronoDuration::seconds(1))))
        .execute(&mut conn)
        .await
        .expect("make arena due");
    arena::start_scheduled_with_presence(tournament.id, |_| HashSet::new(), &mut conn)
        .await
        .expect("start empty arena");
    let seed_players = HashSet::from([players[0], players[1]]);
    for player in players.iter().take(2) {
        let online = seed_players.clone();
        arena::join_with_presence(tournament.id, *player, move |_| online.clone(), &mut conn)
            .await
            .expect("join players for the featured-game wave");
    }

    let pool = db.pool.clone();
    let tournament_id = tournament.id;
    let public_tournament_id = TournamentId(tournament.nanoid.clone());
    let Backend { pid } = diesel::sql_query("SELECT pg_backend_pid() AS pid")
        .get_result(&mut conn)
        .await
        .unwrap();
    let entrants = players[2..].to_vec();
    let joins = conn
        .transaction::<_, DbError, _>(async |tc| {
            Tournament::find_by_tournament_id_for_update(&public_tournament_id, tc).await?;
            let mut joins = JoinSet::new();
            for user_id in &entrants {
                let pool = pool.clone();
                let user_id = *user_id;
                joins.spawn(async move {
                    let mut joining_conn = get_conn(&pool).await.unwrap();
                    arena::join_with_presence(
                        tournament_id,
                        user_id,
                        |_| HashSet::new(),
                        &mut joining_conn,
                    )
                    .await
                });
            }
            let mut observer = get_conn(&pool).await.expect("get lock observer connection");
            timeout(Duration::from_secs(10), async {
                loop {
                    let blocked = diesel::sql_query(
                        "WITH RECURSIVE blocked(pid) AS ( \
                            SELECT pid FROM pg_stat_activity \
                            WHERE datname = current_database() \
                            AND $1 = ANY(pg_blocking_pids(pid)) \
                            UNION \
                            SELECT waiting.pid FROM pg_stat_activity waiting \
                            JOIN blocked blocker \
                            ON blocker.pid = ANY(pg_blocking_pids(waiting.pid)) \
                            WHERE waiting.datname = current_database() \
                         ) SELECT count(*) AS count FROM blocked",
                    )
                    .bind::<Integer, _>(pid)
                    .get_result::<BlockedJoins>(&mut observer)
                    .await
                    .expect("observe concurrent joins");
                    if blocked.count == entrants.len() as i64 {
                        break;
                    }
                    sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("all six joins overlap the pairing-wave transaction");
            let wave =
                arena::pair_waiting_with_presence(tournament_id, |_| seed_players.clone(), tc)
                    .await?;
            assert_eq!(wave.new_games.len(), 1);
            assert_eq!(
                Tournament::find(tournament_id, tc).await?.featured_game_id,
                Some(wave.new_games[0].id),
            );
            Ok(joins)
        })
        .await
        .expect("commit the featured-game change");
    timeout(Duration::from_secs(10), async move {
        let mut joins = joins;
        while let Some(join) = joins.join_next().await {
            assert!(
                join.unwrap()
                    .expect("every concurrent arena join succeeds")
                    .joined_now
            );
        }
    })
    .await
    .expect("queued joins complete after the wave commits");

    let memberships = TournamentUser::find_by_tournament_id(tournament_id, &mut conn)
        .await
        .expect("load arena entrants");
    assert_eq!(
        memberships
            .iter()
            .map(|row| row.user_id)
            .collect::<HashSet<_>>(),
        players.into_iter().collect(),
    );
    assert_eq!(
        memberships
            .iter()
            .map(|row| row.pairing_number)
            .collect::<Vec<_>>(),
        (0..8).map(Some).collect::<Vec<_>>(),
    );
}
