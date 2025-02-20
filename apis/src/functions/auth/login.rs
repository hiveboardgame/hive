use crate::responses::AccountResponse;
use leptos::prelude::*;

#[server]
pub async fn login(
    email: String,
    password: String,
    pathname: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::db::pool;
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    use db_lib::get_conn;
    use db_lib::models::User;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user: User = User::find_by_email(&email, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&user.password).map_err(ServerFnError::new)?;
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => {

            let req: actix_web::HttpRequest = leptos_actix::extract().await?;
            Identity::login(&req.extensions(), user.id.to_string())?;
            leptos_actix::redirect(&pathname);
            AccountResponse::from_uuid(&user.id, &mut conn).await
        }
        Err(_) => Err(ServerFnError::new("Password does not match.")),
    }
}
