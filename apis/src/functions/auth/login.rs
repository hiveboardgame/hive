use crate::functions::accounts::account_response::AccountResponse;
use leptos::*;

#[server]
pub async fn login(
    username: String,
    password: String,
    pathname: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::db::pool;
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    use db_lib::models::user::User;
    let pool = pool()?;
    let user: User = User::find_by_username(&username, &pool)
        .await
        .map_err(ServerFnError::new)?;

    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&user.password).map_err(ServerFnError::new)?;
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => {
            let req = use_context::<actix_web::HttpRequest>()
                .ok_or("Failed to get HttpRequest")
                .map_err(ServerFnError::new)?;
            Identity::login(&req.extensions(), user.id.to_string())?;
            leptos_actix::redirect(&pathname);
            AccountResponse::from_uuid(&user.id, &pool).await
        }
        Err(_) => Err(ServerFnError::new("Password does not match.")),
    }
}
