use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GameId, TournamentId};

pub const MAX_CHAT_MESSAGE_LENGTH: usize = 1000;

fn is_allowed_chat_character(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\t') || !c.is_control()
}

fn truncate_chat_message(text: &mut String) {
    if text.chars().count() <= MAX_CHAT_MESSAGE_LENGTH {
        return;
    }
    *text = text.chars().take(MAX_CHAT_MESSAGE_LENGTH).collect();
}

pub fn normalize_chat_message(text: &str) -> String {
    let mut normalized: String = text
        .chars()
        .filter(|c| is_allowed_chat_character(*c))
        .collect();
    truncate_chat_message(&mut normalized);
    normalized
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

    pub fn parse_slug(value: &str) -> Option<Self> {
        match value {
            "players" => Some(Self::Players),
            "spectators" => Some(Self::Spectators),
            _ => None,
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

    pub const fn global() -> Self {
        Self::Global
    }

    pub fn from_destination(destination: &ChatDestination) -> Self {
        match destination {
            ChatDestination::User((other_user_id, _)) => Self::direct(*other_user_id),
            ChatDestination::GamePlayers(game_id) => Self::game_players(game_id),
            ChatDestination::GameSpectators(game_id) => Self::game_spectators(game_id),
            ChatDestination::TournamentLobby(tournament_id) => Self::tournament(tournament_id),
            ChatDestination::Global => Self::global(),
        }
    }

    pub fn error_field(&self) -> String {
        match self {
            Self::Direct(other_user_id) => format!("chat:direct:{other_user_id}"),
            Self::Tournament(tournament_id) => format!("chat:tournament:{}", tournament_id.0),
            Self::Game { game_id, thread } => {
                format!("chat:game:{}:{}", game_id.0, thread.slug())
            }
            Self::Global => "chat:global".to_string(),
        }
    }

    pub fn from_error_field(field: &str) -> Option<Self> {
        let mut parts = field.splitn(4, ':');
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some("chat"), Some("direct"), Some(other_user_id), None) => {
                Uuid::parse_str(other_user_id).ok().map(Self::direct)
            }
            (Some("chat"), Some("tournament"), Some(tournament_id), None) => {
                Some(Self::Tournament(TournamentId(tournament_id.to_string())))
            }
            (Some("chat"), Some("game"), Some(game_id), Some(thread)) => {
                GameThread::parse_slug(thread).map(|thread| Self::Game {
                    game_id: GameId(game_id.to_string()),
                    thread,
                })
            }
            (Some("chat"), Some("global"), None, None) => Some(Self::Global),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SimpleDestination {
    User,
    Game,
    Tournament(TournamentId),
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatDestination {
    User((Uuid, String)),
    GamePlayers(GameId),
    GameSpectators(GameId),
    TournamentLobby(TournamentId),
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: Option<i64>,
    pub user_id: Uuid,
    pub username: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub message: String,
    pub turn: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessageContainer {
    pub destination: ChatDestination,
    pub message: ChatMessage,
    pub client_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatHistoryResponse {
    Messages(Vec<ChatMessage>),
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
    pub fn new(destination: ChatDestination, message: &ChatMessage) -> Self {
        Self::new_with_client_id(destination, message, None)
    }

    pub fn new_with_client_id(
        destination: ChatDestination,
        message: &ChatMessage,
        client_id: Option<Uuid>,
    ) -> Self {
        Self {
            destination,
            message: message.to_owned(),
            client_id,
        }
    }

    pub fn time(&mut self) {
        self.message.time();
    }
}

impl ChatMessage {
    pub fn new(
        username: String,
        user_id: Uuid,
        message: &str,
        timestamp: Option<DateTime<Utc>>,
        turn: Option<usize>,
    ) -> Self {
        Self {
            id: None,
            username,
            user_id,
            message: normalize_chat_message(message),
            timestamp,
            turn,
        }
    }

    pub fn time(&mut self) {
        self.timestamp = Some(Utc::now());
    }

    pub fn normalize(&mut self) {
        self.message = normalize_chat_message(&self.message);
    }
}
