use super::game_reaction::GameReaction;
use crate::responses::{ChallengeResponse, GameResponse, TournamentResponse, UserResponse};
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use shared_types::{ChallengeId, ChatMessageContainer};
use shared_types::{GameId, TournamentId};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerResult {
    Ok(Box<ServerMessage>),
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
    Game(Box<GameUpdate>),
    Challenge(ChallengeUpdate),
    UserSearch(Vec<UserResponse>),
    UserStatus(UserUpdate),
    Tournament(TournamentUpdate),
    // sent to everyone in the game when a user joins the game
    Join(UserResponse),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TournamentUpdate {
    Created(TournamentResponse),
    Deleted(TournamentId),
    Modified(TournamentResponse),
    Joined(TournamentResponse),
    Left(TournamentResponse),
    Tournaments(Vec<TournamentResponse>),
    Invited(TournamentResponse),
    Declined(TournamentResponse),
    Uninvited(TournamentResponse),
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
    pub game_id: GameId,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeUpdate {
    Created(ChallengeResponse),         // A new challenge was created
    Removed(ChallengeId),               // A challenge was removed
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
