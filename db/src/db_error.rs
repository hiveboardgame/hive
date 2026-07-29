use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DbError {
    #[error("Tournament does not have enough players")]
    NotEnoughPlayers,
    #[error("Tournament is full")]
    TournamentFull,
    #[error("Cannot join an invite only tournament")]
    TournamentInviteOnly,
    #[error("Invalid TournamentDetails: {info}")]
    InvalidTournamentDetails { info: String },
    #[error("Internal database error: {reason}")]
    InternalError { reason: String },
    #[error("Chat client ID conflicts with an existing message")]
    ChatClientIdConflict,
    #[error("Invalid input")]
    InvalidInput { info: String, error: String },
    #[error("Invalid action: {info}")]
    InvalidAction { info: String },
    #[error("Not found: {reason}")]
    NotFound { reason: String },
    #[error("Time not present: {reason}")]
    TimeNotFound { reason: String },
    #[error("Game is over")]
    GameIsOver,
    #[error("You are not authorized to perform that action")]
    Unauthorized,
}

impl From<diesel::result::Error> for DbError {
    fn from(err: diesel::result::Error) -> DbError {
        match err {
            diesel::result::Error::NotFound => DbError::NotFound {
                reason: "Not found.".to_string(),
            },
            // Keeping the cause: without it every failed query anywhere in the
            // crate surfaces as the same three words, in logs and in tests.
            error => DbError::InternalError {
                reason: error.to_string(),
            },
        }
    }
}

impl From<shared_types::ChallengeError> for DbError {
    fn from(err: shared_types::ChallengeError) -> DbError {
        match err {
            shared_types::ChallengeError::NotValidTimeMode { found } => {
                DbError::TimeNotFound { reason: found }
            }
            error => DbError::InternalError {
                reason: error.to_string(),
            },
        }
    }
}

impl From<shared_types::GameQueryValidationError> for DbError {
    fn from(err: shared_types::GameQueryValidationError) -> Self {
        DbError::InvalidInput {
            info: err.to_string(),
            error: String::new(),
        }
    }
}
