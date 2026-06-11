use crate::{
    functions::games::get::server_fn::codec,
    responses::{GameBatchResponse, GameResponse, RatingHistoryResponse},
};
use leptos::prelude::*;
use shared_types::{GameId, GameSpeed, GamesQueryOptions};
use uuid::Uuid;

#[server(input = codec::Cbor, output = codec::Cbor, client = crate::client::ApiClient)]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_uuid(game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor, client = crate::client::ApiClient)]
pub async fn get_game_from_nanoid(game_id: GameId) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_game_id(&game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor, client = crate::client::ApiClient)]
pub async fn get_batch_from_options(
    options: GamesQueryOptions,
) -> Result<GameBatchResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::batch_from_options(options, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

/// Routing hint for "cold-launched with no other context": returns the game
/// the user should land on if they tapped the app icon (or, transitively
/// during the YourTurn-only push era, tapped a notification whose deep link
/// got swallowed by the plugin). See `apis/src/providers/launch_router.rs`
/// for the caller. None means there's nothing urgent and the homepage stays.
#[server(input = codec::Cbor, output = codec::Cbor, client = crate::client::ApiClient)]
pub async fn most_urgent_game() -> Result<Option<GameId>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::Game};
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let game = Game::most_urgent_for_user(user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    Ok(game.map(|g| GameId(g.nanoid)))
}

#[server(input = codec::Cbor, output = codec::Cbor, client = crate::client::ApiClient)]
pub async fn get_rating_history_resource(
    user_id: Uuid,
    game_speed: GameSpeed,
) -> Result<Vec<RatingHistoryResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    RatingHistoryResponse::get_rating_history_from_uuid_and_speed(&user_id, &game_speed, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
