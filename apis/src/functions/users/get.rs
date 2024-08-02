use crate::responses::UserResponse;
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
pub async fn username_taken(username: String) -> Result<bool, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::User;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    Ok(User::username_exists(&username, &mut conn).await?)
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
            UserResponse::from_model(user, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        )
    }
    Ok(results)
}
