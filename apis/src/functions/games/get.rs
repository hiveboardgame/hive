use crate::responses::game::GameResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::game::Game;
    let pool = pool()?;
    let game = Game::find_by_uuid(&game_id, &pool).await?;
    GameResponse::new_from_db(&game, &pool)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_game_from_nanoid(nanoid: String) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::game::Game;
    let pool = pool()?;
    let game = Game::find_by_nanoid(&nanoid, &pool).await?;
    GameResponse::new_from_db(&game, &pool)
        .await
        .map_err(ServerFnError::new)
}
