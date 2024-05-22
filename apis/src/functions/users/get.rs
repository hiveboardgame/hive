use crate::responses::{GameResponse, UserResponse};
use chrono::{DateTime, Utc};
use leptos::*;
use shared_types::GameSpeed;
use uuid::Uuid;

#[server]
pub async fn get_user_by_uuid(uuid: Uuid) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    UserResponse::from_uuid(&uuid, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_user_by_username(username: String) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    UserResponse::from_username(&username, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn username_taken(username: String) -> Result<bool, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::User;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    Ok(User::username_exists(&username, &mut conn).await?)
}

#[server]
pub async fn get_ongoing_games(username: String) -> Result<Vec<GameResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Game;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let games: Vec<Game> = Game::get_ongoing_games_for_username(&username, &mut conn).await?;
    let mut results: Vec<GameResponse> = Vec::new();
    for game in games.iter() {
        if let Ok(game_response) = GameResponse::new_from_model(game, &mut conn).await {
            results.push(game_response);
        }
    }
    Ok(results)
}

#[server]
pub async fn get_finished_games_in_batches(
    username: String,
    last_timestamp: Option<DateTime<Utc>>,
    last_id: Option<Uuid>,
    amount: i64,
) -> Result<(Vec<GameResponse>, bool), ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Game;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let games: Vec<Game> = Game::get_x_finished_games_for_username(
        &username,
        &mut conn,
        last_timestamp,
        last_id,
        amount,
    )
    .await?;
    let mut results: Vec<GameResponse> = Vec::new();
    let got_amount = games.len() as i64 == amount;
    for game in games.iter() {
        if let Ok(game_response) = GameResponse::new_from_model(game, &mut conn).await {
            results.push(game_response);
        }
    }
    Ok((results, got_amount))
}

#[server]
pub async fn get_top_users(
    game_speed: GameSpeed,
    limit: i64,
) -> Result<Vec<UserResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::{Rating, User};
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let top_users: Vec<(User, Rating)> = User::get_top_users(&game_speed, limit, &mut conn).await?;
    let mut results: Vec<UserResponse> = Vec::new();
    for (user, _rating) in top_users.iter() {
        results.push(
            UserResponse::from_user(user, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        )
    }
    Ok(results)
}
