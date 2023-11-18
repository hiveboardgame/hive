use crate::common::game_action::GameAction;
use crate::functions::games::game_response::GameStateResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    game_action: GameAction,
    game: GameStateResponse,
    game_id: String, // nanoid
    user_id: Uuid,
    username: String,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {

use db_lib::{models::game::Game, DbPool};
use leptos::ServerFnError;

impl ServerMessage {
    pub async fn new(game_id: String, game_action: GameAction, user_id: Uuid, username: String, pool: DbPool) -> Result<ServerMessage, ServerFnError> {
        Ok(ServerMessage {
            game_action,
            game: GameStateResponse::new_from_nanoid(&game_id, &pool).await?,
            game_id,
            user_id,
            username,
        })
    }
}

}}
