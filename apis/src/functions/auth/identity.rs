use actix_identity::Identity;
use actix_web::{web::Data, HttpRequest};
use leptos::prelude::*;
use uuid::Uuid;

use crate::api::v1::auth::{decode::jwt_decode, jwt_secret::JwtSecret};

pub async fn identity() -> Result<Identity, ServerFnError> {
    leptos_actix::extract().await?
}

/// Resolve the current user's UUID from either:
/// 1. `Authorization: Bearer <jwt>` header — used by the Apiary mobile app
///    (CSR build, cross-origin so cookies don't flow reliably).
/// 2. `actix-identity` session cookie — used by the SSR + hydrate path
///    (same-origin, HttpOnly cookie).
///
/// Bearer is checked first so a client that sends both still gets a clean
/// answer. Both code paths converge on the same `Uuid`, so server functions
/// downstream of `uuid()` don't need to care which auth mode was used.
pub async fn uuid() -> Result<Uuid, ServerFnError> {
    let req: HttpRequest = leptos_actix::extract().await?;
    if let Some(uuid) = uuid_from_bearer(&req) {
        return Ok(uuid);
    }

    let id_str = identity().await?.id()?;
    Uuid::parse_str(&id_str)
        .map_err(|e| ServerFnError::new(format!("Could not retrieve Uuid from identity: {e}")))
}

fn uuid_from_bearer(req: &HttpRequest) -> Option<Uuid> {
    let header = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let token = header.strip_prefix("Bearer ")?;
    let secret = req.app_data::<Data<JwtSecret>>()?;
    let sub = jwt_decode(token, &secret.decoding).ok()?;
    Uuid::parse_str(&sub).ok()
}

#[cfg(feature = "ssr")]
pub async fn ensure_admin(conn: &mut db_lib::DbConn<'_>) -> Result<(), ServerFnError> {
    use db_lib::models::User;
    let user = User::find_by_uuid(&uuid().await?, conn).await?;
    if !user.admin {
        Err(ServerFnError::new("You are not an admin"))
    } else {
        Ok(())
    }
}
