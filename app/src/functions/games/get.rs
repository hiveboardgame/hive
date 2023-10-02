use super::game_response::GameStateResponse;
use leptos::*;

#[server(GetGame)]
pub async fn get_game(game_id: i32) -> Result<GameStateResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::game::Game;
    let pool = pool()?;
    let game = Game::get(game_id, &pool).await?;
    GameStateResponse::new_from_db(&game, &pool).await
}
