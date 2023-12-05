use http::*;
use http_serde;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::functions::{
    games::game_response::GameStateResponse, users::user_response::UserResponse,
};

use super::game_action::GameAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerResult {
    Ok(ServerOk),
    Err(ExternalServerError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServerError {
    // TODO: this needs user_id or another way to only send this to the user who caused the error
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
pub struct GameActionResponse {
    pub game_action: GameAction,
    pub game: GameStateResponse,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerOk {
    GameRequiresAction(Vec<String>),
    GameUpdate(GameActionResponse),
    UserStatusChange(UserUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserUpdate {
    Online(UserResponse),
    Offline(UserResponse),
    Away(UserResponse),
    EloChange(UserResponse),
}
