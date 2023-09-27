use diesel::result::Error as DieselError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("invalid field {field}: {reason}")]
    UserInputError { field: String, reason: String },
    #[error("Database error: {0}")]
    DatabaseError(#[from] DieselError),
}
