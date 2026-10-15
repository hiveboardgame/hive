use crate::models::PersistedConfigError;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DbError {
    #[error("Serializable transaction conflicted with concurrent work")]
    SerializationConflict,
    #[error("A tournament with that name already exists")]
    TournamentNameTaken,
    #[error("Tournament does not have enough players")]
    NotEnoughPlayers,
    #[error("Tournament is full")]
    TournamentFull,
    #[error("Cannot join an invite only tournament")]
    TournamentInviteOnly,
    #[error("Invalid TournamentDetails: {info}")]
    InvalidTournamentDetails { info: String },
    #[error("Invalid persisted tournament state: {reason}")]
    InvalidPersistedTournament { reason: String },
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

impl From<DieselError> for DbError {
    fn from(err: DieselError) -> DbError {
        match err {
            DieselError::NotFound => DbError::NotFound {
                reason: "Not found.".to_string(),
            },
            DieselError::DatabaseError(DatabaseErrorKind::SerializationFailure, _) => {
                DbError::SerializationConflict
            }
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info)
                if info.constraint_name() == Some("tournaments_name_key") =>
            {
                DbError::TournamentNameTaken
            }
            DieselError::DeserializationError(error)
                if error.downcast_ref::<PersistedConfigError>().is_some() =>
            {
                DbError::InvalidPersistedTournament {
                    reason: error.to_string(),
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error};

    struct ConstraintInfo(&'static str);

    impl DatabaseErrorInformation for ConstraintInfo {
        fn message(&self) -> &str {
            "unique violation"
        }
        fn details(&self) -> Option<&str> {
            None
        }
        fn hint(&self) -> Option<&str> {
            None
        }
        fn table_name(&self) -> Option<&str> {
            Some("tournaments")
        }
        fn column_name(&self) -> Option<&str> {
            Some("name")
        }
        fn constraint_name(&self) -> Option<&str> {
            Some(self.0)
        }
        fn statement_position(&self) -> Option<i32> {
            None
        }
    }

    #[test]
    fn tournament_name_constraint_maps_to_typed_error_only() {
        let name_error = Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(ConstraintInfo("tournaments_name_key")),
        );
        assert!(matches!(
            DbError::from(name_error),
            DbError::TournamentNameTaken
        ));

        let other_error = Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(ConstraintInfo("tournaments_nanoid")),
        );
        assert!(matches!(
            DbError::from(other_error),
            DbError::InternalError { .. }
        ));
    }

    #[test]
    fn malformed_persisted_tournament_configuration_is_typed() {
        let error =
            Error::DeserializationError(Box::new(PersistedConfigError(String::from("bad config"))));
        assert!(matches!(
            DbError::from(error),
            DbError::InvalidPersistedTournament { reason } if reason.contains("bad config")
        ));
    }
}
