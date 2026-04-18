use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

use crate::{GameId, TournamentId};

const MAX_CHAT_MESSAGE_LENGTH: usize = 1000;

fn is_allowed_chat_character(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\t') || !c.is_control()
}

fn normalize_chat_message(text: &str) -> String {
    let mut normalized: String = text
        .chars()
        .filter(|c| is_allowed_chat_character(*c))
        .collect();
    truncate_chat_message(&mut normalized);
    normalized
}

fn truncate_chat_message(text: &mut String) {
    if text.len() <= MAX_CHAT_MESSAGE_LENGTH {
        return;
    }

    let boundary = (0..=3)
        .find_map(|offset| {
            MAX_CHAT_MESSAGE_LENGTH
                .checked_sub(offset)
                .filter(|&idx| text.is_char_boundary(idx))
        })
        .unwrap_or(0);
    text.truncate(boundary);
}

/// Channel type names used for persistent chat (must match db schema).
pub const CHANNEL_TYPE_GAME_PLAYERS: &str = "game_players";
pub const CHANNEL_TYPE_GAME_SPECTATORS: &str = "game_spectators";
pub const CHANNEL_TYPE_TOURNAMENT_LOBBY: &str = "tournament_lobby";
pub const CHANNEL_TYPE_DIRECT: &str = "direct";
pub const CHANNEL_TYPE_GLOBAL: &str = "global";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    GamePlayers,
    GameSpectators,
    TournamentLobby,
    Direct,
    Global,
}

impl ChannelType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GamePlayers => CHANNEL_TYPE_GAME_PLAYERS,
            Self::GameSpectators => CHANNEL_TYPE_GAME_SPECTATORS,
            Self::TournamentLobby => CHANNEL_TYPE_TOURNAMENT_LOBBY,
            Self::Direct => CHANNEL_TYPE_DIRECT,
            Self::Global => CHANNEL_TYPE_GLOBAL,
        }
    }
}

impl fmt::Display for ChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChannelType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            CHANNEL_TYPE_GAME_PLAYERS => Ok(Self::GamePlayers),
            CHANNEL_TYPE_GAME_SPECTATORS => Ok(Self::GameSpectators),
            CHANNEL_TYPE_TOURNAMENT_LOBBY => Ok(Self::TournamentLobby),
            CHANNEL_TYPE_DIRECT => Ok(Self::Direct),
            CHANNEL_TYPE_GLOBAL => Ok(Self::Global),
            _ => Err(()),
        }
    }
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

    const fn global() -> Self {
        Self::Global
    }

    pub fn from_destination(destination: &ChatDestination) -> Self {
        match destination {
            ChatDestination::TournamentLobby(tournament_id) => Self::tournament(tournament_id),
            ChatDestination::User((other_user_id, _)) => Self::direct(*other_user_id),
            ChatDestination::GamePlayers(game_id) => Self::game_players(game_id),
            ChatDestination::GameSpectators(game_id) => Self::game_spectators(game_id),
            ChatDestination::Global => Self::global(),
        }
    }

    pub fn persistent_key(&self, current_user_id: Option<Uuid>) -> Option<PersistentChannelKey> {
        match self {
            Self::Direct(other_user_id) => {
                current_user_id.map(|user_id| PersistentChannelKey::direct(user_id, *other_user_id))
            }
            Self::Tournament(tournament_id) => {
                Some(PersistentChannelKey::tournament(tournament_id))
            }
            Self::Game { game_id, thread } => match thread {
                GameThread::Players => Some(PersistentChannelKey::game_players(game_id)),
                GameThread::Spectators => Some(PersistentChannelKey::game_spectators(game_id)),
            },
            Self::Global => Some(PersistentChannelKey::global()),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PersistentChannelKey {
    pub channel_type: ChannelType,
    pub channel_id: String,
}

impl PersistentChannelKey {
    fn new(channel_type: ChannelType, channel_id: impl Into<String>) -> Self {
        Self {
            channel_type,
            channel_id: channel_id.into(),
        }
    }

    fn normalized(channel_type: ChannelType, channel_id: impl AsRef<str>) -> Option<Self> {
        let channel_id = channel_id.as_ref();
        match channel_type {
            ChannelType::Direct => Some(Self::new(
                channel_type,
                canonical_direct_channel_id(channel_id)?,
            )),
            ChannelType::Global => Some(Self::global()),
            _ => Some(Self::new(channel_type, channel_id)),
        }
    }

    pub fn from_raw(channel_type: &str, channel_id: impl AsRef<str>) -> Option<Self> {
        Self::normalized(channel_type.parse().ok()?, channel_id)
    }

    pub fn direct(current_user_id: Uuid, other_user_id: Uuid) -> Self {
        Self::new(
            ChannelType::Direct,
            canonical_dm_channel_id(current_user_id, other_user_id),
        )
    }

    pub fn tournament(tournament_id: &TournamentId) -> Self {
        Self::new(ChannelType::TournamentLobby, tournament_id.0.clone())
    }

    pub fn game_players(game_id: &GameId) -> Self {
        Self::new(ChannelType::GamePlayers, game_id.0.clone())
    }

    pub fn game_spectators(game_id: &GameId) -> Self {
        Self::new(ChannelType::GameSpectators, game_id.0.clone())
    }

    fn global() -> Self {
        Self::new(ChannelType::Global, CHANNEL_TYPE_GLOBAL)
    }

    pub fn direct_other_user_id(&self, current_user_id: Uuid) -> Option<Uuid> {
        (self.channel_type == ChannelType::Direct)
            .then(|| other_user_from_dm_channel(&self.channel_id, current_user_id))
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreadCount {
    pub key: ConversationKey,
    pub count: i64,
}

/// Canonical channel_id for a DM between two users (sorted UUIDs so both participants use the same key).
fn canonical_dm_channel_id(a: Uuid, b: Uuid) -> String {
    if a < b {
        format!("{}::{}", a, b)
    } else {
        format!("{}::{}", b, a)
    }
}

fn canonical_direct_channel_id(channel_id: &str) -> Option<String> {
    let (a, b) = parse_direct_channel_users(channel_id)?;
    Some(canonical_dm_channel_id(a, b))
}

fn parse_direct_channel_users(channel_id: &str) -> Option<(Uuid, Uuid)> {
    let mut parts = channel_id.split("::");
    let a = Uuid::parse_str(parts.next()?).ok()?;
    let b = Uuid::parse_str(parts.next()?).ok()?;
    if parts.next().is_some() || a == b {
        return None;
    }
    Some((a, b))
}

/// Returns the other participant if `channel_id` is a canonical DM pair containing `me`.
fn other_user_from_dm_channel(channel_id: &str, me: Uuid) -> Option<Uuid> {
    let (a, b) = parse_direct_channel_users(channel_id)?;
    if channel_id != canonical_dm_channel_id(a, b) {
        return None;
    }

    if a == me {
        Some(b)
    } else if b == me {
        Some(a)
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatDestination {
    User((Uuid, String)),          // user_id, username
    GamePlayers(GameId),           // to players in the game, nanoid
    GameSpectators(GameId),        // to spectators of the game, nanoid
    TournamentLobby(TournamentId), // to tournament lobby
    Global,                        // to everyone if you have superpowers
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub user_id: Uuid,
    pub username: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub message: String,
    pub turn: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessageContainer {
    pub destination: ChatDestination,
    // TODO: @ion maybe even better to change this to messages: Vec<ChatMessage>
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatHistoryResponse {
    Messages(Vec<ChatMessage>),
    AccessDenied,
}

impl ChatMessageContainer {
    pub fn new(destination: ChatDestination, message: &ChatMessage) -> Self {
        Self {
            destination,
            message: message.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::{normalize_chat_message, PersistentChannelKey, MAX_CHAT_MESSAGE_LENGTH};
    use uuid::Uuid;

    #[test]
    fn normalize_chat_message_removes_control_characters_and_truncates() {
        let raw = format!(
            "hello\u{0000}\u{0008}{}",
            "x".repeat(MAX_CHAT_MESSAGE_LENGTH + 10)
        );
        let normalized = normalize_chat_message(&raw);

        assert!(!normalized.contains('\u{0000}'));
        assert!(!normalized.contains('\u{0008}'));
        assert!(normalized.len() <= MAX_CHAT_MESSAGE_LENGTH);
        assert!(normalized.starts_with("hello"));
    }

    #[test]
    fn direct_channel_key_returns_the_other_user() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let key = PersistentChannelKey::direct(a, b);

        assert_eq!(key.direct_other_user_id(a), Some(b));
        assert_eq!(key.direct_other_user_id(b), Some(a));
    }
}
