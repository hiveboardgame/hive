use crate::i18n::*;
use leptos_i18n::I18nContext;
use thiserror::Error;

const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub(crate) enum PasswordError {
    #[error("Passwords don't match.")]
    Mismatch,
    #[error(
        "Password is too short; it must be at least {} characters.",
        MIN_PASSWORD_LENGTH
    )]
    TooShort,
    #[error(
        "Password is too long; it must not exceed {} characters.",
        MAX_PASSWORD_LENGTH
    )]
    TooLong,
}

impl PasswordError {
    pub(crate) fn is_ready_to_display(self, password_blurred: bool, confirmation: &str) -> bool {
        match self {
            Self::TooShort => password_blurred,
            Self::Mismatch => !confirmation.is_empty(),
            Self::TooLong => true,
        }
    }
}

pub(crate) fn validate_password(
    password: &str,
    password_confirmation: &str,
) -> Result<(), PasswordError> {
    let password_length = password.chars().count();
    if password_length < MIN_PASSWORD_LENGTH {
        return Err(PasswordError::TooShort);
    }
    if password_length > MAX_PASSWORD_LENGTH {
        return Err(PasswordError::TooLong);
    }
    if password != password_confirmation {
        return Err(PasswordError::Mismatch);
    }
    Ok(())
}

pub(crate) fn password_error_message(
    i18n: I18nContext<Locale, I18nKeys>,
    error: PasswordError,
) -> String {
    match error {
        PasswordError::TooShort => t_string!(
            i18n,
            common.auth.password_too_short,
            min = MIN_PASSWORD_LENGTH
        )
        .to_string(),
        PasswordError::TooLong => t_string!(
            i18n,
            common.auth.password_too_long,
            max = MAX_PASSWORD_LENGTH
        )
        .to_string(),
        PasswordError::Mismatch => t_string!(i18n, common.auth.passwords_do_not_match).to_string(),
    }
}

pub(crate) fn password_requirements_message(i18n: I18nContext<Locale, I18nKeys>) -> String {
    t_string!(
        i18n,
        user_config.create_account.password_requirements,
        min = MIN_PASSWORD_LENGTH,
        max = MAX_PASSWORD_LENGTH
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_limits_count_unicode_characters() {
        for character in ["a", "é", "Ж", "界", "🦋"] {
            let minimum = character.repeat(MIN_PASSWORD_LENGTH);
            assert!(validate_password(&minimum, &minimum).is_ok());
            let too_short = character.repeat(MIN_PASSWORD_LENGTH - 1);
            assert_eq!(
                validate_password(&too_short, &too_short),
                Err(PasswordError::TooShort)
            );
            let maximum = character.repeat(MAX_PASSWORD_LENGTH);
            assert!(validate_password(&maximum, &maximum).is_ok());
            let too_long = character.repeat(MAX_PASSWORD_LENGTH + 1);
            assert_eq!(
                validate_password(&too_long, &too_long),
                Err(PasswordError::TooLong)
            );
            assert_eq!(
                validate_password(&too_long, ""),
                Err(PasswordError::TooLong)
            );
        }
    }

    #[test]
    fn password_requires_minimum_length_and_matching_confirmation() {
        let password = "a".repeat(MIN_PASSWORD_LENGTH);
        assert!(validate_password(&password, &password).is_ok());
        assert!(validate_password(&password[1..], &password[1..]).is_err());
        assert!(validate_password(&password, &format!("{password}b")).is_err());
    }
}
