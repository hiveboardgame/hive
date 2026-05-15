use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    chat_message::UnreadCount,
    GameId,
    GameThread,
    TournamentChatCapabilities,
    TournamentId,
};

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
    /// Muting tournament chat suppresses unread badges and notifications.
    /// It does not prevent websocket delivery of tournament messages.
    pub muted: bool,
    pub access: TournamentChatCapabilities,
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
    pub muted_tournament_ids: Vec<TournamentId>,
    pub unread_counts: Vec<UnreadCount>,
}
