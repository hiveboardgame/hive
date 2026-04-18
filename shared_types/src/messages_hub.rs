use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GameId, GameThread};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmConversation {
    pub other_user_id: Uuid,
    pub username: String,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TournamentChannel {
    pub nanoid: String,
    pub name: String,
    pub muted: bool,
    pub can_chat: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameChannel {
    pub game_id: GameId,
    pub thread: GameThread,
    pub label: String,
    pub is_player: bool,
    pub finished: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagesHubData {
    pub dms: Vec<DmConversation>,
    pub tournaments: Vec<TournamentChannel>,
    pub games: Vec<GameChannel>,
}
