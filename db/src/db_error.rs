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
    #[error("Invalid TournamentDetails")]
    InvalidTournamentDetails { info: String },
    #[error("Internal database error")]
    InternalError,
    #[error("Invalid input")]
    InvalidInput { info: String, error: String },
    #[error("Invalid action")]
    InvalidAction { info: String },
    #[error("Not found")]
    NotFound { reason: String },
    #[error("Time not present")]
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
            _ => DbError::InternalError,
        }
    }
}

impl From<shared_types::ChallengeError> for DbError {
    fn from(err: shared_types::ChallengeError) -> DbError {
        match err {
            shared_types::ChallengeError::NotValidTimeMode { found } => {
                DbError::TimeNotFound { reason: found }
            }
            _ => DbError::InternalError,
        }
    }
}
