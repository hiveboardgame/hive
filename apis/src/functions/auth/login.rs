use crate::responses::AccountResponse;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// What login() returns to the client.
///
/// - `account`: the user object the frontend uses to populate auth state.
/// - `token`: a JWT for bearer auth, used by cross-origin clients (Apiary
///   mobile, future native clients). SSR + hydrate ignore this — they're
///   same-origin and rely on the Identity cookie set in the same response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub account: AccountResponse,
    pub token: String,
}

#[server(client = crate::client::ApiClient)]
pub async fn login(
    email: String,
    password: String,
    pathname: String,
) -> Result<LoginResponse, ServerFnError> {
    use crate::api::v1::auth::{encode::jwt_encode_user_id, jwt_secret::JwtSecret};
    use crate::functions::db::pool;
    use actix_identity::Identity;
    use actix_web::{web::Data, HttpMessage};
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user_result = User::find_by_email(&email, &mut conn)
        .await
        .map_err(ServerFnError::new);
    let user = if let Ok(user) = user_result {
        user
    } else {
        User::find_by_username(&email, &mut conn)
            .await
            .map_err(ServerFnError::new)?
    };
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&user.password).map_err(ServerFnError::new)?;
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => {
            let req: actix_web::HttpRequest = leptos_actix::extract().await?;
            Identity::login(&req.extensions(), user.id.to_string())?;
            let jwt_secret = req
                .app_data::<Data<JwtSecret>>()
                .ok_or_else(|| ServerFnError::new("JWT secret not configured"))?;
            let token = jwt_encode_user_id(user.id, &jwt_secret.encoding)
                .map_err(|e| ServerFnError::new(format!("Could not encode JWT: {e}")))?;
            leptos_actix::redirect(&pathname);
            let account = AccountResponse::from_uuid(&user.id, &mut conn).await?;
            Ok(LoginResponse { account, token })
        }
        Err(_) => Err(ServerFnError::new("Password does not match.")),
    }
}
