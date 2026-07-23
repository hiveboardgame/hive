use super::{decode::jwt_decode, jwt_secret::JwtSecret};
use actix_web::{http::header::AUTHORIZATION, HttpRequest};
use db_lib::{get_conn, models::User, DbPool};

/// Resolves the user authenticated by an `Authorization: Bearer <jwt>`
/// header, if present and valid. Shared between the `/ws/` handshake and
/// the `/api/v1/auth/whoami` HTTP endpoint so the two never drift.
pub async fn resolve_bearer_user(
    req: &HttpRequest,
    pool: &DbPool,
    jwt_secret: &JwtSecret,
) -> Option<User> {
    let token = req
        .headers()
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?;

    let email = jwt_decode(token, &jwt_secret.decoding).ok()?;
    let mut conn = get_conn(pool).await.ok()?;
    User::find_by_email(&email, &mut conn).await.ok()
}
