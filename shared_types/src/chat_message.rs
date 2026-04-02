use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GameId, TournamentId};

pub const MAX_CHAT_MESSAGE_LENGTH: usize = 1000;

pub fn truncate_chat_message(text: &mut String) {
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
pub const CHAT_CHANNEL_TYPES: [&str; 5] = [
    CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_GAME_SPECTATORS,
    CHANNEL_TYPE_TOURNAMENT_LOBBY,
    CHANNEL_TYPE_DIRECT,
    CHANNEL_TYPE_GLOBAL,
];

pub fn is_valid_chat_channel_type(channel_type: &str) -> bool {
    CHAT_CHANNEL_TYPES.contains(&channel_type)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectChannelParseError {
    InvalidFormat,
    InvalidUuid,
    NotParticipant,
}

/// Canonical channel_id for a DM between two users (sorted UUIDs so both participants use the same key).
pub fn canonical_dm_channel_id(a: Uuid, b: Uuid) -> String {
    if a < b {
        format!("{}::{}", a, b)
    } else {
        format!("{}::{}", b, a)
    }
}

/// Returns the other participant if `channel_id` is a canonical DM pair containing `me`.
pub fn other_user_from_dm_channel(channel_id: &str, me: Uuid) -> Option<Uuid> {
    let mut parts = channel_id.split("::");
    let a = Uuid::parse_str(parts.next()?).ok()?;
    let b = Uuid::parse_str(parts.next()?).ok()?;
    if parts.next().is_some() {
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

/// For a user and a DM channel id input (single UUID or UUID pair), return canonical channel id.
pub fn canonicalize_dm_channel_id_for_user(channel_id: &str, user_id: Uuid) -> Option<String> {
    if channel_id.contains("::") {
        let other = other_user_from_dm_channel(channel_id, user_id)?;
        Some(canonical_dm_channel_id(user_id, other))
    } else {
        let other_id = Uuid::parse_str(channel_id).ok()?;
        Some(canonical_dm_channel_id(user_id, other_id))
    }
}

/// Parse a direct-channel input from a sender into the recipient UUID.
/// Accepts either `recipient_uuid` or `uuid_a::uuid_b` formats.
pub fn direct_other_user_for_sender(
    channel_id: &str,
    sender_id: Uuid,
) -> Result<Uuid, DirectChannelParseError> {
    if !channel_id.contains("::") {
        return Uuid::parse_str(channel_id).map_err(|_| DirectChannelParseError::InvalidUuid);
    }

    let mut parts = channel_id.split("::");
    let a_raw = parts.next().ok_or(DirectChannelParseError::InvalidFormat)?;
    let b_raw = parts.next().ok_or(DirectChannelParseError::InvalidFormat)?;
    if parts.next().is_some() {
        return Err(DirectChannelParseError::InvalidFormat);
    }

    let a = Uuid::parse_str(a_raw).map_err(|_| DirectChannelParseError::InvalidUuid)?;
    let b = Uuid::parse_str(b_raw).map_err(|_| DirectChannelParseError::InvalidUuid)?;

    if a == sender_id {
        Ok(b)
    } else if b == sender_id {
        Ok(a)
    } else {
        Err(DirectChannelParseError::NotParticipant)
    }
}

/// LIKE patterns used to find DM channels containing a user id.
pub fn dm_channel_like_patterns(user_id: Uuid) -> (String, String) {
    (format!("{user_id}::%"), format!("%::{user_id}"))
}

/// Maps a chat destination and sender to (channel_type, channel_id) for persistent storage.
/// `sender_id` is the user sending the message (used for DMs to build the canonical channel key).
pub fn chat_channel(destination: &ChatDestination, sender_id: Uuid) -> (&'static str, String) {
    match destination {
        ChatDestination::GamePlayers(game_id, _, _) => {
            (CHANNEL_TYPE_GAME_PLAYERS, game_id.0.clone())
        }
        ChatDestination::GameSpectators(game_id, _, _) => {
            (CHANNEL_TYPE_GAME_SPECTATORS, game_id.0.clone())
        }
        ChatDestination::TournamentLobby(tournament_id) => {
            (CHANNEL_TYPE_TOURNAMENT_LOBBY, tournament_id.0.clone())
        }
        ChatDestination::User((other_id, _)) => (
            CHANNEL_TYPE_DIRECT,
            canonical_dm_channel_id(sender_id, *other_id),
        ),
        ChatDestination::Global => (CHANNEL_TYPE_GLOBAL, CHANNEL_TYPE_GLOBAL.to_string()),
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
    User((Uuid, String)),               // user_id, username
    GamePlayers(GameId, Uuid, Uuid),    // to players in the game, nanoid, white uuid, black uuid
    GameSpectators(GameId, Uuid, Uuid), // to spectators of the game, nanoid, white uuid, black uuid
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
        let mut message = message.to_owned();
        truncate_chat_message(&mut message);
        Self {
            username,
            user_id,
            message,
            timestamp,
            turn,
        }
    }

    pub fn time(&mut self) {
        self.timestamp = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_dm_channel_id,
        canonicalize_dm_channel_id_for_user,
        direct_other_user_for_sender,
        DirectChannelParseError,
    };
    use uuid::Uuid;

    #[test]
    fn canonicalize_dm_accepts_single_other_user_uuid() {
        let me = Uuid::new_v4();
        let other = Uuid::new_v4();
        let channel_id = canonicalize_dm_channel_id_for_user(&other.to_string(), me);
        assert_eq!(channel_id, Some(canonical_dm_channel_id(me, other)));
    }

    #[test]
    fn canonicalize_dm_reorders_non_canonical_pair() {
        let me = Uuid::new_v4();
        let other = Uuid::new_v4();
        let reversed = format!("{other}::{me}");
        let channel_id = canonicalize_dm_channel_id_for_user(&reversed, me);
        assert_eq!(channel_id, Some(canonical_dm_channel_id(me, other)));
    }

    #[test]
    fn canonicalize_dm_rejects_pair_without_current_user() {
        let me = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let channel_id = canonicalize_dm_channel_id_for_user(&format!("{a}::{b}"), me);
        assert_eq!(channel_id, None);
    }

    #[test]
    fn direct_other_user_accepts_single_uuid() {
        let me = Uuid::new_v4();
        let other = Uuid::new_v4();
        let parsed = direct_other_user_for_sender(&other.to_string(), me);
        assert_eq!(parsed, Ok(other));
    }

    #[test]
    fn direct_other_user_rejects_pair_without_sender() {
        let me = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let parsed = direct_other_user_for_sender(&format!("{a}::{b}"), me);
        assert_eq!(parsed, Err(DirectChannelParseError::NotParticipant));
    }

    #[test]
    fn direct_other_user_rejects_invalid_pair_format() {
        let me = Uuid::new_v4();
        let parsed = direct_other_user_for_sender("abc::def::ghi", me);
        assert_eq!(parsed, Err(DirectChannelParseError::InvalidFormat));
    }
}
