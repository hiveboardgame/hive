const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 128;

pub fn validate_password(password: &str, password_confirmation: &str) -> Result<(), String> {
    if password != password_confirmation {
        return Err("Passwords don't match.".to_string());
    }
    let password_length = password.len();
    if password_length < MIN_PASSWORD_LENGTH {
        return Err(format!(
            "Password is too short, it must be at least {MIN_PASSWORD_LENGTH}"
        ));
    }
    if password_length > MAX_PASSWORD_LENGTH {
        return Err(format!(
            "Password is too long it must not exceed {MAX_PASSWORD_LENGTH}"
        ));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn hash_password(password: &str) -> Result<String, leptos::prelude::ServerFnError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(leptos::prelude::ServerFnError::new)?
        .to_string())
}

#[cfg(feature = "ssr")]
pub fn verify_password(
    password: &str,
    password_hash: &str,
) -> Result<(), leptos::prelude::ServerFnError> {
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};

    let parsed_hash =
        PasswordHash::new(password_hash).map_err(leptos::prelude::ServerFnError::new)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| leptos::prelude::ServerFnError::new("Password does not match."))
}
