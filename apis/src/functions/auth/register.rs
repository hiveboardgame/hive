use leptos::prelude::*;

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

#[server]
pub async fn register(
    username: String,
    email: String,
    password: String,
    password_confirmation: String,
    pathname: String,
) -> Result<(), ServerFnError> {
    use crate::functions::db::pool;
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    use db_lib::db_error::DbError;
    use db_lib::get_conn;
    use db_lib::models::{NewUser, User};
    use diesel_async::scoped_futures::ScopedFutureExt;
    use diesel_async::AsyncConnection;

    validate_password(&password, &password_confirmation).map_err(ServerFnError::new)?;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(ServerFnError::new)?
        .to_string();
    let email = email.to_lowercase();
    let new_user = NewUser::new(&username, &password, &email)?;

    let user = conn
        .transaction::<_, DbError, _>(move |tc| {
            async move { User::create(new_user, tc).await }.scope_boxed()
        })
        .await?;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    Identity::login(&req.extensions(), user.id.to_string()).expect("To have logged in");
    leptos_actix::redirect(&pathname);

    Ok(())
}
