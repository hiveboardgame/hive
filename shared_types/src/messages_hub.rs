use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ConversationUnreadState, GameId, TournamentId};

pub const MESSAGES_HUB_SECTION_LIMIT: usize = 50;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmConversation {
    pub other_user_id: Uuid,
    pub username: String,
    pub peer_deleted: bool,
    pub last_message_id: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TournamentChannel {
    pub tournament_id: TournamentId,
    pub name: String,
    pub last_message_id: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameChannel {
    pub game_id: GameId,
    pub label: String,
    pub finished: bool,
    pub last_message_id: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatInboxSnapshot {
    pub blocked_user_ids: Vec<Uuid>,
    pub muted_tournament_ids: Vec<TournamentId>,
    pub unread_states: Vec<ConversationUnreadState>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagesCatalogData {
    pub dms: Vec<DmConversation>,
    pub tournaments: Vec<TournamentChannel>,
    pub games: Vec<GameChannel>,
}
