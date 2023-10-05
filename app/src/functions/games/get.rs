use super::game_response::GameStateResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameStateResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::game::Game;
    let pool = pool()?;
    let game = Game::find_by_uuid(&game_id, &pool).await?;
    GameStateResponse::new_from_db(&game, &pool).await
}

#[server]
pub async fn get_game_from_url(url: String) -> Result<GameStateResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::game::Game;
    let pool = pool()?;
    let game = Game::find_by_url(&url, &pool).await?;
    GameStateResponse::new_from_db(&game, &pool).await
}
