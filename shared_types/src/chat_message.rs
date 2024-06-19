use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::GameId;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[derive(Debug, Clone)]
pub enum SimpleDestination {
    User,
    Game,
    Tournament,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatDestination {
    User((Uuid, String)),               // user_id, username
    GamePlayers(GameId, Uuid, Uuid),    // to players in the game, nanoid, white uuid, black uuid
    GameSpectators(GameId, Uuid, Uuid), // to spectators of the game, nanoid, white uuid, black uuid
    TournamentLobby(String),            // to tournament lobby
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
