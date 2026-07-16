use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GameId, TournamentId};

pub const MAX_CHAT_MESSAGE_LENGTH: usize = 1000;

fn is_allowed_chat_character(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\t') || !c.is_control()
}

pub fn normalize_chat_message(text: &str) -> String {
    text.chars()
        .filter(|character| is_allowed_chat_character(*character))
        .take(MAX_CHAT_MESSAGE_LENGTH)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameThread {
    Players,
    Spectators,
}

impl GameThread {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Players => "players",
            Self::Spectators => "spectators",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversationKey {
    Direct(Uuid),
    Tournament(TournamentId),
    Game { game_id: GameId, thread: GameThread },
    Global,
}

impl ConversationKey {
    pub const fn direct(other_user_id: Uuid) -> Self {
        Self::Direct(other_user_id)
    }

    pub fn tournament(tournament_id: &TournamentId) -> Self {
        Self::Tournament(tournament_id.clone())
    }

    pub fn game(game_id: &GameId, thread: GameThread) -> Self {
        Self::Game {
            game_id: game_id.clone(),
            thread,
        }
    }

    pub fn game_players(game_id: &GameId) -> Self {
        Self::game(game_id, GameThread::Players)
    }

    pub fn game_spectators(game_id: &GameId) -> Self {
        Self::game(game_id, GameThread::Spectators)
    }

    pub fn tracks_read_receipts(&self) -> bool {
        matches!(
            self,
            Self::Direct(_)
                | Self::Tournament(_)
                | Self::Game {
                    thread: GameThread::Players,
                    ..
                }
        )
    }

    pub fn applies_block_filter(&self) -> bool {
        !matches!(self, Self::Global)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: i64,
    pub user_id: Uuid,
    pub username: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub turn: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessageContainer {
    pub key: ConversationKey,
    pub message: ChatMessage,
    pub client_id: Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatHistoryPage {
    pub messages: Vec<ChatMessage>,
    pub next_before_message_id: Option<i64>,
    pub initial_unread_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatHistoryResponse {
    Page(ChatHistoryPage),
    AccessDenied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationUnreadState {
    pub key: ConversationKey,
    pub count: i64,
    pub latest_message_id: i64,
    pub latest_unread_message_id: i64,
    pub last_read_message_id: i64,
}

impl ChatMessageContainer {
    pub fn new(key: ConversationKey, message: ChatMessage, client_id: Uuid) -> Self {
        Self {
            key,
            message,
            client_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_chat_message, ConversationKey, MAX_CHAT_MESSAGE_LENGTH};
    use crate::{GameId, GameThread, TournamentId};
    use uuid::Uuid;

    #[test]
    fn normalization_filters_and_truncates_without_changing_valid_bodies() {
        assert_eq!(normalize_chat_message("valid\nmessage"), "valid\nmessage",);
        assert_eq!(normalize_chat_message("invalid\u{0000}body"), "invalidbody",);

        let oversized = "x".repeat(MAX_CHAT_MESSAGE_LENGTH + 1);
        assert_eq!(
            normalize_chat_message(&oversized),
            "x".repeat(MAX_CHAT_MESSAGE_LENGTH),
        );
    }

    #[test]
    fn only_global_chat_bypasses_block_filtering() {
        assert!(!ConversationKey::Global.applies_block_filter());
        assert!(ConversationKey::Direct(Uuid::new_v4()).applies_block_filter());
        assert!(
            ConversationKey::Tournament(TournamentId("event".to_string())).applies_block_filter()
        );
        assert!(ConversationKey::Game {
            game_id: GameId("game".to_string()),
            thread: GameThread::Players,
        }
        .applies_block_filter());
    }
}
