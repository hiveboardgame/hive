use crate::responses::GameResponse;
use leptos::prelude::*;
use shared_types::{GameId, GamesQueryOptions};
use uuid::Uuid;
use crate::functions::games::get::server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_uuid(game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_from_nanoid(game_id: GameId) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_game_id(&game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_batch_from_options(options: GamesQueryOptions) -> Result<Vec<GameResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::vec_from_options(options, &mut conn).await.map_err(ServerFnError::new)
}
