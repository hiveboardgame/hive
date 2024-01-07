use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DbError {
    #[error("Internal database error")]
    InternalError,
    #[error("Invalid input")]
    InvalidInput { info: String, error: String },
    #[error("Not found")]
    NotFound { reason: String },
    #[error("Time not present")]
    TimeNotFound { reason: String },
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
