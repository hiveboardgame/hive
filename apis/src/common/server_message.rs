use crate::common::game_action::GameAction;
use crate::responses::GameResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    pub game_action: GameAction,
    pub game: GameResponse,
    pub game_id: String, // nanoid
    pub user_id: Uuid,
    pub username: String,
}

use cfg_if::cfg_if;

cfg_if! { if #[cfg(feature = "ssr")] {


impl ServerMessage {
}

}}
