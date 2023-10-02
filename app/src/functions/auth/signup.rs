use leptos::*;

#[server(Signup, "/api")]
pub async fn signup(
    username: String,
    email: String,
    password: String,
    password_confirmation: String,
) -> Result<(), ServerFnError> {
    use crate::functions::db::pool;
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    use db_lib::models::user::User;
    use rand_core::OsRng;
    use uuid::Uuid;

    if password != password_confirmation {
        return Err(ServerFnError::ServerError(
            "Passwords don't match.".to_string(),
        ));
    }

    let pool = pool()?;
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .to_string();

    let uid = Uuid::new_v4().to_string();
    let user = User::new(&uid, &username, &hashed_password, &email)?;
    user.insert(&pool).await?;
    let user = User::find_by_uid(&user.uid, &pool).await?;
    let req = use_context::<actix_web::HttpRequest>()
        .ok_or("Failed to get HttpRequest")
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Identity::login(&req.extensions(), user.uid.to_string()).unwrap();
    leptos_actix::redirect("/");

    Ok(())
}
