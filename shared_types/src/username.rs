use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_USERNAME_LENGTH: usize = 20;
pub const MIN_USERNAME_LENGTH: usize = 2;
pub const DELETED_USERNAME_PREFIX: &str = "deleted_user_";
/// Language-independent fallback for exports and server-generated labels.
pub const DELETED_USER_DISPLAY_NAME: &str = "Deleted user";
const RESERVED_USERNAMES: [&str; 4] = ["black", "white", "admin", "me"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum UsernameError {
    #[error("Username has invalid characters")]
    InvalidCharacters,
    #[error("Username is too long.")]
    TooLong,
    #[error("Username is too short.")]
    TooShort,
    #[error("Username is reserved.")]
    Reserved,
    #[error("Username cannot start with {}.", DELETED_USERNAME_PREFIX)]
    DeletedPrefix,
}

pub fn validate_username(username: &str) -> Result<(), UsernameError> {
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(UsernameError::InvalidCharacters);
    }
    if username.len() > MAX_USERNAME_LENGTH {
        return Err(UsernameError::TooLong);
    }
    if username.len() < MIN_USERNAME_LENGTH {
        return Err(UsernameError::TooShort);
    }
    let normalized = username.to_ascii_lowercase();
    if RESERVED_USERNAMES.contains(&normalized.as_str()) {
        return Err(UsernameError::Reserved);
    }
    if normalized.starts_with(DELETED_USERNAME_PREFIX) {
        return Err(UsernameError::DeletedPrefix);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tombstone_namespace_cannot_be_registered() {
        for username in ["deleted_user_999", "DeLeTeD_UsEr_9"] {
            assert_eq!(
                validate_username(username),
                Err(UsernameError::DeletedPrefix)
            );
        }
    }
}
