mod common;

use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Game, NewGame, NewUser, Rating, User},
    schema::ratings,
};
use diesel::{
    prelude::*,
    sql_types::{Bool, Text},
    QueryableByName,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use hive_lib::{Color, GameControl, GameStatus, GameType};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};
use std::time::Duration;
use tokio::sync::oneshot;

const FINALIZER_APPLICATION_NAME: &str = "rating_update_lock_test_finalizer";
const FIRST_GAME_FINALIZER_APPLICATION_NAME: &str = "game_finalization_lock_test_first";
const SECOND_GAME_FINALIZER_APPLICATION_NAME: &str = "game_finalization_lock_test_second";

#[derive(QueryableByName)]
struct LockWait {
    #[diesel(sql_type = Bool)]
    is_waiting: bool,
}

#[tokio::test(flavor = "multi_thread")]
async fn game_finalization_reads_rating_rows_after_waiting_for_locks() {
    let db = common::db::test_db().await;
    let mut setup_conn = get_conn(&db.pool).await.expect("get setup connection");
    let white = create_user("alice", &mut setup_conn).await;
    let black = create_user("bob", &mut setup_conn).await;
    let game = create_bullet_game(white.id, black.id, &mut setup_conn).await;

    let (rating_locked_tx, rating_locked_rx) = oneshot::channel();
    let (release_rating_lock_tx, release_rating_lock_rx) = oneshot::channel();
    let lock_pool = db.pool.clone();
    let white_id = white.id;

    let rating_lock_task = tokio::spawn(async move {
        let mut conn = get_conn(&lock_pool)
            .await
            .expect("get rating lock connection");
        conn.transaction::<_, DbError, _>(move |tc| {
            async move {
                diesel::update(
                    ratings::table
                        .filter(ratings::user_uid.eq(white_id))
                        .filter(ratings::speed.eq(GameSpeed::Bullet.to_string())),
                )
                .set(ratings::rating.eq(1800.0))
                .execute(tc)
                .await?;

                rating_locked_tx.send(()).expect("signal rating lock");
                release_rating_lock_rx.await.expect("release rating lock");
                Ok(())
            }
            .scope_boxed()
        })
        .await
    });

    rating_locked_rx.await.expect("rating row is locked");

    let finalizer_pool = db.pool.clone();
    let finalizer_task = tokio::spawn(async move {
        let mut conn = get_conn(&finalizer_pool)
            .await
            .expect("get finalizer connection");
        set_application_name(FINALIZER_APPLICATION_NAME, &mut conn).await;
        game.resign(&GameControl::Resign(Color::Black), &mut conn)
            .await
    });

    wait_for_finalizer_to_wait_on_lock(&db.pool).await;
    release_rating_lock_tx
        .send(())
        .expect("release rating row lock");

    rating_lock_task
        .await
        .expect("join rating lock task")
        .expect("rating lock transaction");
    let finalized_game = finalizer_task
        .await
        .expect("join finalizer task")
        .expect("finalize game");

    let white_rating_at_finalization = finalized_game.white_rating;
    common::db::truncate(&db.pool).await;
    assert_eq!(white_rating_at_finalization, Some(1800.0));
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_game_finalization_does_not_apply_ratings_twice() {
    let db = common::db::test_db().await;
    let mut setup_conn = get_conn(&db.pool).await.expect("get setup connection");
    let white = create_user("charlie", &mut setup_conn).await;
    let black = create_user("diana", &mut setup_conn).await;
    let stale_game = create_bullet_game(white.id, black.id, &mut setup_conn).await;

    let (rating_locked_tx, rating_locked_rx) = oneshot::channel();
    let (release_rating_lock_tx, release_rating_lock_rx) = oneshot::channel();
    let lock_pool = db.pool.clone();
    let white_id = white.id;

    let rating_lock_task = tokio::spawn(async move {
        let mut conn = get_conn(&lock_pool)
            .await
            .expect("get rating lock connection");
        conn.transaction::<_, DbError, _>(move |tc| {
            async move {
                diesel::update(
                    ratings::table
                        .filter(ratings::user_uid.eq(white_id))
                        .filter(ratings::speed.eq(GameSpeed::Bullet.to_string())),
                )
                .set(ratings::rating.eq(1800.0))
                .execute(tc)
                .await?;

                rating_locked_tx.send(()).expect("signal rating lock");
                release_rating_lock_rx.await.expect("release rating lock");
                Ok(())
            }
            .scope_boxed()
        })
        .await
    });

    rating_locked_rx.await.expect("rating row is locked");

    let first_finalizer_pool = db.pool.clone();
    let first_stale_game = stale_game.clone();
    let first_finalizer_task = tokio::spawn(async move {
        let mut conn = get_conn(&first_finalizer_pool)
            .await
            .expect("get first finalizer connection");
        set_application_name(FIRST_GAME_FINALIZER_APPLICATION_NAME, &mut conn).await;
        first_stale_game
            .resign(&GameControl::Resign(Color::Black), &mut conn)
            .await
    });

    wait_for_backend_to_wait_on_lock(&db.pool, FIRST_GAME_FINALIZER_APPLICATION_NAME).await;

    let second_finalizer_pool = db.pool.clone();
    let second_finalizer_task = tokio::spawn(async move {
        let mut conn = get_conn(&second_finalizer_pool)
            .await
            .expect("get second finalizer connection");
        set_application_name(SECOND_GAME_FINALIZER_APPLICATION_NAME, &mut conn).await;
        stale_game
            .resign(&GameControl::Resign(Color::Black), &mut conn)
            .await
    });

    wait_for_backend_to_wait_on_lock(&db.pool, SECOND_GAME_FINALIZER_APPLICATION_NAME).await;
    release_rating_lock_tx
        .send(())
        .expect("release rating row lock");

    rating_lock_task
        .await
        .expect("join rating lock task")
        .expect("rating lock transaction");
    let first_result = first_finalizer_task
        .await
        .expect("join first finalizer task");
    let second_result = second_finalizer_task
        .await
        .expect("join second finalizer task");

    assert!(first_result.expect("first finalization succeeds").finished);
    assert_eq!(
        second_result.expect_err("second stale finalizer should fail"),
        DbError::GameIsOver
    );

    let white_rating = Rating::for_uuid(&white.id, &GameSpeed::Bullet, &mut setup_conn)
        .await
        .expect("load white rating");
    let black_rating = Rating::for_uuid(&black.id, &GameSpeed::Bullet, &mut setup_conn)
        .await
        .expect("load black rating");

    common::db::truncate(&db.pool).await;
    assert_eq!(white_rating.played, 1);
    assert_eq!(white_rating.won, 1);
    assert_eq!(white_rating.lost, 0);
    assert_eq!(black_rating.played, 1);
    assert_eq!(black_rating.won, 0);
    assert_eq!(black_rating.lost, 1);
}

async fn set_application_name(application_name: &str, conn: &mut db_lib::DbConn<'_>) {
    diesel::sql_query(format!("SET application_name = '{application_name}'"))
        .execute(conn)
        .await
        .expect("set application name");
}

async fn create_user(username: &str, conn: &mut db_lib::DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

async fn create_bullet_game(
    white_id: uuid::Uuid,
    black_id: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Game {
    let now = Utc::now();
    let time_left = Some(60 * 1_000_000_000_i64);
    Game::create(
        NewGame {
            nanoid: nanoid::nanoid!(12),
            current_player_id: white_id,
            black_id,
            finished: false,
            game_status: GameStatus::InProgress.to_string(),
            game_type: GameType::MLP.to_string(),
            history: String::from("wQ -;bQ /wQ;"),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: false,
            turn: 2,
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: Some(now),
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
        },
        conn,
    )
    .await
    .expect("insert game")
}

async fn wait_for_finalizer_to_wait_on_lock(pool: &db_lib::DbPool) {
    wait_for_backend_to_wait_on_lock(pool, FINALIZER_APPLICATION_NAME).await;
}

async fn wait_for_backend_to_wait_on_lock(pool: &db_lib::DbPool, application_name: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let mut conn = get_conn(pool)
            .await
            .expect("get pg_stat_activity connection");
        let LockWait { is_waiting } = diesel::sql_query(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM pg_stat_activity
                WHERE application_name = $1
                    AND wait_event_type = 'Lock'
            ) AS is_waiting
            "#,
        )
        .bind::<Text, _>(application_name)
        .get_result(&mut conn)
        .await
        .expect("query backend lock wait");

        if is_waiting {
            return;
        }

        assert!(
            tokio::time::Instant::now() < deadline,
            "{application_name} did not wait on a database lock"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
