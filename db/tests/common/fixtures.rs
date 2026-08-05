#![allow(dead_code)]

use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Game, NewGame, NewUser, Rating, User},
    schema::ratings::{self, deviation, played, rating, speed, user_uid},
    DbConn,
    DbPool,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{Color, GameControl, GameStatus, GameType};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};
use uuid::Uuid;

const CLOCK_SECONDS: i64 = 3600;

pub async fn create_user(username: &str, bot: bool, conn: &mut DbConn<'_>) -> User {
    let mut new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    new_user.bot = bot;
    User::create(new_user, conn).await.expect("insert user")
}

pub async fn rating_for(user: &User, game_speed: &GameSpeed, conn: &mut DbConn<'_>) -> Rating {
    Rating::for_uuid(&user.id, game_speed, conn)
        .await
        .expect("load rating")
}

pub async fn bullet_rating(user: &User, conn: &mut DbConn<'_>) -> Rating {
    rating_for(user, &GameSpeed::Bullet, conn).await
}

pub async fn set_rating(
    user: &User,
    game_speed: &GameSpeed,
    new_rating: f64,
    new_deviation: f64,
    new_played: i64,
    conn: &mut DbConn<'_>,
) {
    diesel::update(
        ratings::table
            .filter(user_uid.eq(user.id))
            .filter(speed.eq(game_speed.to_string())),
    )
    .set((
        rating.eq(new_rating),
        deviation.eq(new_deviation),
        played.eq(new_played),
    ))
    .execute(conn)
    .await
    .expect("set rating fixture");
}

pub fn new_game(white_id: Uuid, black_id: Uuid, game_speed: GameSpeed, rated: bool) -> NewGame {
    let now = Utc::now();
    let time_left = Some(CLOCK_SECONDS * 1_000_000_000);
    let timeout_at = time_left.map(|nanos| now + chrono::Duration::nanoseconds(nanos));
    NewGame {
        nanoid: nanoid::nanoid!(12),
        current_player_id: white_id,
        black_id,
        finished: false,
        game_status: GameStatus::InProgress.to_string(),
        game_type: GameType::MLP.to_string(),
        history: String::from("wQ -;bQ /wQ;"),
        game_control_history: String::new(),
        rated,
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
        time_base: Some(CLOCK_SECONDS as i32),
        time_increment: Some(0),
        last_interaction: Some(now),
        black_time_left: time_left,
        white_time_left: time_left,
        speed: game_speed.to_string(),
        hashes: Vec::new(),
        conclusion: Conclusion::Unknown.to_string(),
        tournament_id: None,
        tournament_game_result: TournamentGameResult::Unknown.to_string(),
        game_start: GameStart::Moves.to_string(),
        move_times: Vec::new(),
        timeout_at,
    }
}

pub fn new_bullet_game(white_id: Uuid, black_id: Uuid) -> NewGame {
    new_game(white_id, black_id, GameSpeed::Bullet, true)
}

pub async fn create_game(new_game: NewGame, conn: &mut DbConn<'_>) -> Game {
    Game::create(new_game, conn).await.expect("insert game")
}

pub async fn create_bullet_game(white_id: Uuid, black_id: Uuid, conn: &mut DbConn<'_>) -> Game {
    create_game(new_bullet_game(white_id, black_id), conn).await
}

pub async fn resign_as(game: Game, color: Color, pool: &DbPool) -> Result<Game, DbError> {
    let mut conn = get_conn(pool).await.expect("get finalizer connection");
    conn.transaction::<_, DbError, _>(async move |tc| {
        game.resign(&GameControl::Resign(color), tc).await
    })
    .await
}

pub async fn resign_as_white(game: Game, pool: &DbPool) -> Result<Game, DbError> {
    resign_as(game, Color::White, pool).await
}

pub async fn draw_game(game: Game, pool: &DbPool) -> Result<Game, DbError> {
    let mut conn = get_conn(pool).await.expect("get finalizer connection");
    conn.transaction::<_, DbError, _>(async move |tc| {
        let offered = game
            .write_game_control(&GameControl::DrawOffer(Color::White), tc)
            .await?;
        offered
            .accept_draw(&GameControl::DrawAccept(Color::Black), tc)
            .await
    })
    .await
}
