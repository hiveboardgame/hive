use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GameId, TournamentId};

const MAX_MESSAGE_LENGTH: usize = 1000;

/// Channel type names used for persistent chat (must match db schema).
pub const CHANNEL_TYPE_GAME_PLAYERS: &str = "game_players";
pub const CHANNEL_TYPE_GAME_SPECTATORS: &str = "game_spectators";
pub const CHANNEL_TYPE_TOURNAMENT_LOBBY: &str = "tournament_lobby";
pub const CHANNEL_TYPE_DIRECT: &str = "direct";
pub const CHANNEL_TYPE_GLOBAL: &str = "global";

/// Canonical channel_id for a DM between two users (sorted UUIDs so both participants use the same key).
pub fn canonical_dm_channel_id(a: Uuid, b: Uuid) -> String {
    if a < b {
        format!("{}::{}", a, b)
    } else {
        format!("{}::{}", b, a)
    }
}

/// Maps a chat destination and sender to (channel_type, channel_id) for persistent storage.
/// `sender_id` is the user sending the message (used for DMs to build the canonical channel key).
pub fn chat_channel(destination: &ChatDestination, sender_id: Uuid) -> (&'static str, String) {
    match destination {
        ChatDestination::GamePlayers(game_id, _, _) => (CHANNEL_TYPE_GAME_PLAYERS, game_id.0.clone()),
        ChatDestination::GameSpectators(game_id, _, _) => {
            (CHANNEL_TYPE_GAME_SPECTATORS, game_id.0.clone())
        }
        ChatDestination::TournamentLobby(tournament_id) => {
            (CHANNEL_TYPE_TOURNAMENT_LOBBY, tournament_id.0.clone())
        }
        ChatDestination::User((other_id, _)) => (CHANNEL_TYPE_DIRECT, canonical_dm_channel_id(sender_id, *other_id)),
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
        message.truncate(MAX_MESSAGE_LENGTH);
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
