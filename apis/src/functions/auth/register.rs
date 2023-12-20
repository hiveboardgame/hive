use leptos::*;

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
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    use db_lib::models::user::{NewUser, User};
    use rand_core::OsRng;

    if password != password_confirmation {
        return Err(ServerFnError::ServerError(
            "Passwords don't match.".to_string(),
        ));
    }

    let pool = pool()?;
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .to_string();

    let new_user = NewUser::new(&username, &password, &email)?;
    let user = User::create(&new_user, &pool).await?;
    let req = use_context::<actix_web::HttpRequest>()
        .ok_or("Failed to get HttpRequest")
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Identity::login(&req.extensions(), user.id.to_string()).expect("To have logged in");
    leptos_actix::redirect(&pathname);

    Ok(())
}
