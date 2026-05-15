use crate::responses::UserResponse;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::codec;
use shared_types::GameSpeed;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ResolvedUsername {
    pub username: String,
    pub uid: Uuid,
}

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

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_top_users(
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
        User::get_top_users(&game_speed, maybe_user, limit, &mut conn).await?;
    let mut results: Vec<(usize, UserResponse)> = Vec::new();
    for (user, _rating, rank) in top_users.iter() {
        results.push((
            *rank as usize,
            UserResponse::from_model(user, &mut conn)
                .await
                .map_err(ServerFnError::new)?,
        ))
    }
    Ok(results)
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
pub async fn resolve_username(username: String) -> Result<ResolvedUsername, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_username(&username, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(ResolvedUsername {
        username: user.username,
        uid: user.id,
    })
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
