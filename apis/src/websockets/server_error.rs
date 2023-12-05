use diesel::result::Error as DieselError;
use hive_lib::game_error::GameError;
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Generic error: {reason}")]
    GenericError { reason: String },
    // #[error("Authentication error: {0}")]
    // AuthenticationError(#[from] AuthenticationError),
    #[error("invalid field {field}: {reason}")]
    UserInputError { field: String, reason: String },
    #[error("Hive game error: {0}")]
    GameError(#[from] GameError),
    #[error("Internal hive game error: {0}")]
    InternalGameError(GameError),
    #[error("Database error: {0}")]
    DatabaseError(#[from] DieselError),
    // #[error("Challenge error: {0}")]
    // ChallengeError(#[from] ChallengeError),
    #[error("Unimplemented")]
    Unimplemented,
}

impl ServerError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::GenericError { reason: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::GameError(_) => StatusCode::BAD_REQUEST,
            Self::InternalGameError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            // Self::AuthenticationError(err) => match err {
            //     AuthenticationError::MissingToken => StatusCode::UNAUTHORIZED,
            //     AuthenticationError::Forbidden => StatusCode::FORBIDDEN,
            //     AuthenticationError::MalformedJWT(_) | AuthenticationError::MissingSubject => {
            //         StatusCode::BAD_REQUEST
            //     }
            //     AuthenticationError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            //     AuthenticationError::InvalidJWT(_) => StatusCode::UNAUTHORIZED,
            // },
            Self::UserInputError {
                field: _,
                reason: _,
            } => StatusCode::BAD_REQUEST,
            Self::DatabaseError(err) => match err {
                // DbError => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            // Self::ChallengeError(err) => match err {
            //     ChallengeError::MissingChallenger(_) => StatusCode::INTERNAL_SERVER_ERROR,
            //     ChallengeError::OwnChallenge => StatusCode::BAD_REQUEST,
            // },
            Self::Unimplemented => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
