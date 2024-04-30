use super::game_reaction::GameReaction;
use crate::responses::challenge::ChallengeResponse;
use crate::responses::game::GameResponse;
use crate::responses::user::UserResponse;
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use shared_types::chat_message::ChatMessageContainer;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerResult {
    Ok(ServerMessage),
    Err(ExternalServerError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServerError {
    pub user_id: Uuid,
    pub field: String,
    pub reason: String,
    #[serde(with = "http_serde::status_code")]
    pub status_code: StatusCode,
}

impl fmt::Display for ExternalServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: We encountered an error {} because {} ",
            self.status_code, self.field, self.reason
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Pong {
        ping_sent: DateTime<Utc>,
        pong_sent: DateTime<Utc>,
    },
    ConnectionUpdated(Uuid, String),
    Chat(Vec<ChatMessageContainer>),
    Game(GameUpdate),
    Challenge(ChallengeUpdate),
    UserStatus(UserUpdate),
    // sent to everyone in the game when a user joins the game
    Join(UserResponse),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameUpdate {
    Reaction(GameActionResponse),
    Urgent(Vec<GameResponse>),
    Tv(GameResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameActionResponse {
    pub game_action: GameReaction,
    pub game: GameResponse,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeUpdate {
    Created(ChallengeResponse),         // A new challenge was created
    Removed(String),                    // A challenge was removed
    Direct(ChallengeResponse),          // Player got directly invited to a game
    Challenges(Vec<ChallengeResponse>), //
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdate {
    pub status: UserStatus,
    pub user: Option<UserResponse>,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Online,
    Offline,
    Away,
}
