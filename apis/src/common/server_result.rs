use super::game_action::GameAction;
use crate::responses::challenge::ChallengeResponse;
use crate::responses::game::GameResponse;
use crate::responses::user::UserResponse;
use http::StatusCode;
use http_serde;
use serde::{Deserialize, Serialize};
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

// TODO: This might be able to be removed and ServerMessage just goes into ServerResult::Ok(...)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalServerMessage {
    pub destination: MessageDestination,
    pub message: ServerMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDestination {
    Direct(Uuid), // to a user
    Game(String), // to everyone in the game
    Global,       // to everyone online
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    ConnectionUpdated(Uuid, String),
    Chat {
        // Sends a chat message to the game/lobby
        username: String,
        game_id: String,
        message: String,
    },
    GameActionNotification(Vec<GameResponse>),
    GameUpdate(GameActionResponse),
    GameTimeoutCheck(GameResponse),
    GameNew(GameResponse),
    Challenge(ChallengeUpdate),
    UserStatus(UserUpdate),
    // sent to everyone in the game when a user joins the game
    Join(UserResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeUpdate {
    Created(ChallengeResponse),         // A new challenge was created
    Removed(String),                    // A challenge was removed
    Direct(ChallengeResponse),          // Player got directly invited to a game
    Challenges(Vec<ChallengeResponse>), //
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameActionResponse {
    pub game_action: GameAction,
    pub game: GameResponse,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
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
