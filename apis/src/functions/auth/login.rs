use crate::responses::AccountResponse;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// What login() returns to the client.
///
/// - `account`: the user object the frontend uses to populate auth state.
/// - `token`: a JWT for bearer auth, used by cross-origin clients (HiveGame
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
    use crate::{
        api::v1::auth::{encode::jwt_encode_user_id, jwt_secret::JwtSecret},
        functions::{auth::password::verify_password, db::pool},
    };
    use actix_identity::Identity;
    use actix_web::{web::Data, HttpMessage};
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_for_login(&email, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    verify_password(&password, &user.password)?;
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
