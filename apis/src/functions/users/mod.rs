use crate::responses::UserResponse;
use leptos::prelude::*;
use server_fn::codec;
use shared_types::GameSpeed;
#[cfg(feature = "ssr")]
use shared_types::LeaderboardKind;
use uuid::Uuid;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_user_by_uuid(uuid: Uuid) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    UserResponse::from_uuid(&uuid, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn username_taken(username: String) -> Result<bool, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    Ok(User::username_exists(&username, &mut conn).await?)
}

#[cfg(feature = "ssr")]
async fn leaderboard(
    kind: LeaderboardKind,
    game_speed: GameSpeed,
    limit: i64,
) -> Result<Vec<(usize, UserResponse)>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        get_conn,
        models::{Rating, User},
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let maybe_user = uuid().await.ok();
    let top_users: Vec<(User, Rating, i64)> =
        User::get_top_users(kind, &game_speed, maybe_user, limit, &mut conn).await?;
    let users: Vec<User> = top_users.iter().map(|(user, _, _)| user.clone()).collect();
    let mut responses = UserResponse::from_models(&users, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    top_users
        .iter()
        .map(|(user, rating, rank)| {
            responses
                .remove(&user.id)
                .map(|response| (*rank as usize, response))
                .ok_or_else(|| {
                    log::warn!(
                        "Leaderboard row for user {} at speed {} could not be resolved; ratings has no unique (user_uid, speed) constraint",
                        user.id,
                        rating.speed
                    );
                    ServerFnError::new(format!("Duplicate leaderboard row for user {}", user.id))
                })
        })
        .collect()
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_top_users(
    game_speed: GameSpeed,
    limit: i64,
) -> Result<Vec<(usize, UserResponse)>, ServerFnError> {
    leaderboard(LeaderboardKind::Humans, game_speed, limit).await
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_top_bots(
    game_speed: GameSpeed,
    limit: i64,
) -> Result<Vec<(usize, UserResponse)>, ServerFnError> {
    leaderboard(LeaderboardKind::Bots, game_speed, limit).await
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_profile(username: String) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    UserResponse::from_username(&username, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn search_users(pattern: String) -> Result<Vec<UserResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    UserResponse::search_usernames(&pattern, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
