#![cfg(feature = "ssr")]

mod common;

use apis::websocket::server_handlers::tournaments::join::JoinHandler;
use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{NewTournament, NewUser, Tournament, TournamentUser, User},
    schema::tournaments,
};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use shared_types::{
    tournament::{
        round_robin::Config as RoundRobinConfig,
        BotAdmission,
        Clock,
        Config,
        FormatConfig,
        RealtimeClock,
    },
    TournamentDetails,
    TournamentId,
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
async fn concurrent_player_joins_wait_and_preserve_capacity() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let mut users = Vec::new();
    for index in 0..8 {
        let username = format!("burst_join_{index}");
        let user = User::create(
            NewUser::new(&username, "password", &format!("{username}@example.test")).unwrap(),
            &mut conn,
        )
        .await
        .unwrap();
        users.push(user.id);
    }
    let new = NewTournament::new(TournamentDetails {
        name: String::from("Concurrent player joins"),
        description: None,
        seats: Some(6),
        min_seats: 2,
        invite_only: false,
        band_lower: None,
        band_upper: None,
        starts_at: None,
        configuration: Config {
            bot_admission: BotAdmission::HumansOnly,
            format: FormatConfig::RoundRobin(RoundRobinConfig::standard(
                NonZeroU32::new(1).unwrap(),
                Clock::Realtime(RealtimeClock {
                    base_seconds: NonZeroU32::new(300).unwrap(),
                    increment_seconds: 3,
                }),
            )),
        },
    })
    .unwrap();
    let tournament = Tournament::create(users[0], &new, &mut conn).await.unwrap();
    let tournament_id = TournamentId(tournament.nanoid.clone());
    let Backend { pid } = diesel::sql_query("SELECT pg_backend_pid() AS pid")
        .get_result(&mut conn)
        .await
        .unwrap();
    let pool = db.pool.clone();
    let entrants = users[1..7].to_vec();
    let joins = conn
        .transaction::<_, DbError, _>(async |tc| {
            Tournament::find_by_tournament_id_for_update(&tournament_id, tc).await?;
            let mut joins = JoinSet::new();
            for user_id in &entrants {
                let handler = JoinHandler::new(tournament_id.clone(), *user_id, &pool);
                joins.spawn(async move { handler.handle().await });
            }
            let mut observer = get_conn(&pool).await.unwrap();
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
                    .unwrap();
                    if blocked.count == entrants.len() as i64 {
                        break;
                    }
                    sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("all six real handlers overlap at the tournament lock");
            diesel::update(tournaments::table.find(tournament.id))
                .set(tournaments::updated_at.eq(Utc::now()))
                .execute(tc)
                .await?;
            Ok(joins)
        })
        .await
        .unwrap();
    timeout(Duration::from_secs(10), async move {
        let mut joins = joins;
        while let Some(join) = joins.join_next().await {
            join.unwrap().expect("every valid concurrent join succeeds");
        }
    })
    .await
    .expect("the queued joins complete after the lock is released");
    assert_eq!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .into_iter()
            .map(|member| member.user_id)
            .collect::<HashSet<_>>(),
        entrants.into_iter().collect(),
    );
    let Err(error) = JoinHandler::new(tournament_id, users[7], &db.pool)
        .handle()
        .await
    else {
        panic!("a full tournament rejects another entrant")
    };
    assert!(matches!(
        error.downcast_ref::<DbError>(),
        Some(DbError::TournamentFull)
    ));
    assert_eq!(
        TournamentUser::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .unwrap()
            .len(),
        6,
    );
}
