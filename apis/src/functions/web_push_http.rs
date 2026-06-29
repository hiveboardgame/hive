use crate::{functions::devices::new_web_push_device, notifications::web_push};
use actix_identity::Identity;
use actix_web::{get, http::header, post, web, HttpResponse, Responder};
use db_lib::{get_conn, models::PushDevice, DbPool};
use serde::Deserialize;
use uuid::Uuid;

#[get("/api/push/vapid-public-key")]
pub async fn vapid_public_key() -> impl Responder {
    match web_push::cached_public_key() {
        Some(k) => HttpResponse::Ok()
            .insert_header((header::CONTENT_TYPE, "text/plain; charset=utf-8"))
            .body(k.to_string()),
        None => HttpResponse::NoContent().finish(),
    }
}

#[derive(Deserialize)]
pub struct WebSubscription {
    endpoint: String,
    p256dh: String,
    auth: String,
    locale: Option<String>,
    #[serde(default)]
    old_endpoint: Option<String>,
}

#[post("/api/push/web-subscription")]
pub async fn web_subscription(
    identity: Option<Identity>,
    pool: web::Data<DbPool>,
    body: web::Json<WebSubscription>,
) -> impl Responder {
    let Some(identity) = identity else {
        return HttpResponse::Unauthorized().finish();
    };
    let user_id = match identity.id().ok().and_then(|s| Uuid::parse_str(&s).ok()) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().finish(),
    };

    let mut conn = match get_conn(&pool).await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let body = body.into_inner();
    let new_device = match new_web_push_device(
        user_id,
        body.endpoint,
        body.p256dh,
        body.auth,
        env!("CARGO_PKG_VERSION").to_string(),
        body.locale.unwrap_or_else(|| "en".to_string()),
    ) {
        Ok(device) => device,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    match PushDevice::upsert_rotated(new_device, body.old_endpoint, &mut conn).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
