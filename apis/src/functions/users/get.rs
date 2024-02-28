use crate::responses::game::GameResponse;
use crate::responses::user::UserResponse;
use leptos::*;
use shared_types::game_speed::GameSpeed;
use uuid::Uuid;

#[server]
pub async fn get_user_by_uuid(uuid: Uuid) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool()?;
    UserResponse::from_uuid(&uuid, &pool)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_user_by_username(username: String) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool()?;
    UserResponse::from_username(&username, &pool)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_user_games(username: String) -> Result<Vec<GameResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::{game::Game, user::User};
    let pool = pool()?;
    let games: Vec<Game> = User::find_by_username(&username, &pool)
        .await?
        .get_games(&pool)
        .await?;
    let mut results: Vec<GameResponse> = Vec::new();
    for game in games.iter() {
        if let Ok(game_response) = GameResponse::new_from_db(game, &pool).await {
            results.push(game_response);
        }
    }
    Ok(results)
}

#[server]
pub async fn get_top_users(
    game_speed: GameSpeed,
    limit: i64,
) -> Result<Vec<UserResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::{rating::Rating, user::User};
    let pool = pool()?;
    let top_users: Vec<(User, Rating)> = User::get_top_users(&game_speed, limit, &pool).await?;
    let mut results: Vec<UserResponse> = Vec::new();
    for (user, _rating) in top_users.iter() {
        results.push(
            UserResponse::from_user(user, &pool)
                .await
                .map_err(ServerFnError::new)?,
        )
    }
    Ok(results)
}
