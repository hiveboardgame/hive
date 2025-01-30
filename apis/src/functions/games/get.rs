use crate::responses::GameResponse;
use leptos::prelude::*;
use shared_types::GameId;
use uuid::Uuid;

#[server]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_uuid(game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_game_from_nanoid(game_id: GameId) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_game_id(&game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
