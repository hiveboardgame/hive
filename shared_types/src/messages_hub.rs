use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ConversationUnreadState,
    GameChatCapabilities,
    GameId,
    TournamentChatCapabilities,
    TournamentId,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmConversation {
    pub other_user_id: Uuid,
    pub username: String,
    pub peer_deleted: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TournamentChannel {
    pub tournament_id: TournamentId,
    pub name: String,
    pub access: TournamentChatCapabilities,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameChannel {
    pub game_id: GameId,
    pub label: String,
    pub access: GameChatCapabilities,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagesHubData {
    pub dms: Vec<DmConversation>,
    pub tournaments: Vec<TournamentChannel>,
    pub games: Vec<GameChannel>,
    pub muted_tournament_ids: Vec<TournamentId>,
    pub unread_states: Vec<ConversationUnreadState>,
}
