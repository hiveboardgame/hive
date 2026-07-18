mod common;

use chrono::{Duration, Utc};
use db_lib::{
    get_conn,
    models::{Game, NewGame, NewUser, User},
};
use hive_lib::{GameStatus, GameType};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};

#[tokio::test(flavor = "multi_thread")]
async fn active_clock_count_tracks_all_unfinished_realtime_in_progress_rows() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get test connection");
    let white = create_user("clock-white", &mut conn).await;
    let black = create_user("clock-black", &mut conn).await;
    let now = Utc::now();

    create_game(
        white.id,
        black.id,
        false,
        GameStatus::InProgress,
        TimeMode::RealTime,
        Some(now + Duration::minutes(1)),
        &mut conn,
    )
    .await;
    assert_active_clock_count(1, &mut conn).await;

    create_game(
        white.id,
        black.id,
        false,
        GameStatus::InProgress,
        TimeMode::RealTime,
        Some(now - Duration::minutes(1)),
        &mut conn,
    )
    .await;
    assert_active_clock_count(2, &mut conn).await;

    create_game(
        white.id,
        black.id,
        false,
        GameStatus::InProgress,
        TimeMode::RealTime,
        None,
        &mut conn,
    )
    .await;
    assert_active_clock_count(3, &mut conn).await;

    create_game(
        white.id,
        black.id,
        false,
        GameStatus::InProgress,
        TimeMode::Correspondence,
        Some(now + Duration::minutes(1)),
        &mut conn,
    )
    .await;
    assert_active_clock_count(3, &mut conn).await;

    create_game(
        white.id,
        black.id,
        false,
        GameStatus::NotStarted,
        TimeMode::RealTime,
        Some(now + Duration::minutes(1)),
        &mut conn,
    )
    .await;
    assert_active_clock_count(3, &mut conn).await;

    create_game(
        white.id,
        black.id,
        true,
        GameStatus::InProgress,
        TimeMode::RealTime,
        Some(now + Duration::minutes(1)),
        &mut conn,
    )
    .await;
    assert_active_clock_count(3, &mut conn).await;
}

async fn assert_active_clock_count(expected: i64, conn: &mut db_lib::DbConn<'_>) {
    assert_eq!(
        Game::count_active_realtime_clocks(conn)
            .await
            .expect("count active clocks"),
        expected
    );
}

async fn create_user(username: &str, conn: &mut db_lib::DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

async fn create_game(
    white_id: uuid::Uuid,
    black_id: uuid::Uuid,
    finished: bool,
    status: GameStatus,
    time_mode: TimeMode,
    timeout_at: Option<chrono::DateTime<Utc>>,
    conn: &mut db_lib::DbConn<'_>,
) {
    let now = Utc::now();
    Game::create(
        NewGame {
            nanoid: nanoid::nanoid!(12),
            current_player_id: white_id,
            black_id,
            finished,
            game_status: status.to_string(),
            game_type: GameType::MLP.to_string(),
            history: String::new(),
            game_control_history: String::new(),
            rated: false,
            tournament_queen_rule: false,
            turn: 0,
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: time_mode.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: Some(now),
            black_time_left: Some(60_000_000_000),
            white_time_left: Some(60_000_000_000),
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at,
        },
        conn,
    )
    .await
    .expect("insert game fixture");
}
