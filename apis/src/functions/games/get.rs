use crate::responses::GameResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool()?;
    GameResponse::new_from_uuid(game_id, &pool)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_game_from_nanoid(nanoid: String) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool()?;
    GameResponse::new_from_nanoid(&nanoid, &pool)
        .await
        .map_err(ServerFnError::new)
}
