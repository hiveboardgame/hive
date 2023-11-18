use super::game_action::GameAction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMessage {
    pub user_id: Uuid,
    pub game_id: String, // nanoid
    pub game_action: GameAction,
}
