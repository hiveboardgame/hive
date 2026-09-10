mod common;

use chrono::{Duration as ChronoDuration, Utc};
use common::tournament::{create_rr_tournament_with_configuration, create_user};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::TournamentUser,
    schema::tournaments,
    tournaments::arena,
};
use diesel::{
    prelude::*,
    sql_types::{Bool, Integer},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use shared_types::{
    tournament::{arena::Config as ArenaConfig, BotAdmission, Config, FormatConfig, RealtimeClock},
    TournamentStatus,
};
use std::{collections::HashSet, num::NonZeroU32, time::Duration};
use tokio::time::{sleep, timeout};

#[derive(QueryableByName)]
struct Backend {
    #[diesel(sql_type = Integer)]
    pid: i32,
}

#[derive(QueryableByName)]
struct LockWait {
    #[diesel(sql_type = Bool)]
    blocked: bool,
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_new_arena_entrants_receive_distinct_pairing_numbers() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get setup connection");
    let organizer = create_user("arena_join_org", &mut conn).await;
    let first = create_user("arena_join_first", &mut conn).await.id;
    let second = create_user("arena_join_second", &mut conn).await.id;
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        "Concurrent arena joins",
        TournamentStatus::NotStarted,
        Some(Utc::now() + ChronoDuration::hours(1)),
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig {
                duration_seconds: NonZeroU32::new(3_600).unwrap(),
                game_clock: RealtimeClock {
                    base_seconds: NonZeroU32::new(180).unwrap(),
                    increment_seconds: 1,
                },
            }),
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

    let pool = db.pool.clone();
    let tournament_id = tournament.id;
    let second_join = conn
        .transaction::<_, DbError, _>(async move |tc| {
            let joined =
                arena::join_with_presence(tournament_id, first, |_| HashSet::new(), tc).await?;
            assert!(joined.joined_now);

            let mut joining_conn = pool
                .get_owned()
                .await
                .expect("get concurrent join connection");
            let Backend { pid } = diesel::sql_query("SELECT pg_backend_pid() AS pid")
                .get_result(&mut joining_conn)
                .await?;
            let second_join = tokio::spawn(async move {
                arena::join_with_presence(
                    tournament_id,
                    second,
                    |_| HashSet::new(),
                    &mut joining_conn,
                )
                .await
            });
            let mut observer = get_conn(&pool).await.expect("get lock observer connection");
            // Keep the first insertion uncommitted until the second join reaches
            // its conflicting lock, so this also reproduces the old unique violation.
            timeout(Duration::from_secs(5), async {
                loop {
                    let LockWait { blocked } = diesel::sql_query(
                        "SELECT cardinality(pg_blocking_pids($1)) > 0 AS blocked",
                    )
                    .bind::<Integer, _>(pid)
                    .get_result(&mut observer)
                    .await
                    .expect("observe concurrent join lock");
                    if blocked {
                        break;
                    }
                    sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("second join overlaps the uncommitted first join");
            Ok(second_join)
        })
        .await
        .expect("commit first arena join");
    let joined = timeout(Duration::from_secs(5), second_join)
        .await
        .expect("second join completes after the first commits")
        .expect("join second task")
        .expect("second valid arena join succeeds");
    assert!(joined.joined_now);

    let memberships = TournamentUser::find_by_tournament_id(tournament_id, &mut conn)
        .await
        .expect("load arena entrants");
    assert_eq!(
        memberships
            .iter()
            .map(|row| row.user_id)
            .collect::<HashSet<_>>(),
        HashSet::from([first, second]),
    );
    assert_eq!(
        memberships
            .iter()
            .map(|row| row.pairing_number)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)],
    );
}
