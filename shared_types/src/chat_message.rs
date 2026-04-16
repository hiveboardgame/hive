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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelKey {
    pub channel_type: ChannelType,
    pub channel_id: String,
}

impl ChannelKey {
    pub fn new(channel_type: ChannelType, channel_id: impl Into<String>) -> Self {
        Self {
            channel_type,
            channel_id: channel_id.into(),
        }
    }

    pub fn normalized(channel_type: ChannelType, channel_id: impl AsRef<str>) -> Option<Self> {
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

    pub fn global() -> Self {
        Self::new(ChannelType::Global, CHANNEL_TYPE_GLOBAL)
    }

    pub fn from_destination(
        destination: &ChatDestination,
        current_user_id: Option<Uuid>,
    ) -> Option<Self> {
        match destination {
            ChatDestination::TournamentLobby(tournament_id) => {
                Some(Self::tournament(tournament_id))
            }
            ChatDestination::User((other_user_id, _)) => {
                current_user_id.map(|user_id| Self::direct(user_id, *other_user_id))
            }
            ChatDestination::GamePlayers(game_id) => Some(Self::game_players(game_id)),
            ChatDestination::GameSpectators(game_id) => Some(Self::game_spectators(game_id)),
            ChatDestination::Global => Some(Self::global()),
        }
    }

    pub fn from_destination_for_user(destination: &ChatDestination, current_user_id: Uuid) -> Self {
        match destination {
            ChatDestination::TournamentLobby(tournament_id) => Self::tournament(tournament_id),
            ChatDestination::User((other_user_id, _)) => {
                Self::direct(current_user_id, *other_user_id)
            }
            ChatDestination::GamePlayers(game_id) => Self::game_players(game_id),
            ChatDestination::GameSpectators(game_id) => Self::game_spectators(game_id),
            ChatDestination::Global => Self::global(),
        }
    }

    pub fn direct_other_user_id(&self, current_user_id: Uuid) -> Option<Uuid> {
        (self.channel_type == ChannelType::Direct)
            .then(|| other_user_from_dm_channel(&self.channel_id, current_user_id))
            .flatten()
    }
}

/// Canonical channel_id for a DM between two users (sorted UUIDs so both participants use the same key).
pub fn canonical_dm_channel_id(a: Uuid, b: Uuid) -> String {
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
pub fn other_user_from_dm_channel(channel_id: &str, me: Uuid) -> Option<Uuid> {
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
    User((Uuid, String)),               // user_id, username
    GamePlayers(GameId),                // to players in the game, nanoid
    GameSpectators(GameId),             // to spectators of the game, nanoid
    TournamentLobby(TournamentId),      // to tournament lobby
    Global,                             // to everyone if you have superpowers
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
    use super::{normalize_chat_message, ChannelKey, ChannelType, MAX_CHAT_MESSAGE_LENGTH};
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
    fn channel_type_round_trips_from_string() {
        let parsed = "game_players".parse::<ChannelType>();
        assert_eq!(parsed, Ok(ChannelType::GamePlayers));
        assert_eq!(ChannelType::GamePlayers.to_string(), "game_players");
    }

    #[test]
    fn direct_channel_key_returns_the_other_user() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let key = ChannelKey::direct(a, b);

        assert_eq!(key.direct_other_user_id(a), Some(b));
        assert_eq!(key.direct_other_user_id(b), Some(a));
    }
}
