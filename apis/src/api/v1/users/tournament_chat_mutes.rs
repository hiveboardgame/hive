//! Tournament chat mute API: only the authenticated user (from session) can mute/unmute.
//! POST /api/v1/users/me/tournament-chat-mutes — mute (body: { "tournament_id": "nanoid" }).
//! DELETE /api/v1/users/me/tournament-chat-mutes/{tournament_id} — unmute.

use actix_identity::Identity;
use actix_web::{delete, post, web, HttpResponse};
use db_lib::{
    get_conn,
    helpers::{mute_tournament_chat, unmute_tournament_chat},
    DbPool,
};
use serde::Deserialize;
use uuid::Uuid;

fn user_id_from_identity(identity: Option<&Identity>) -> Option<Uuid> {
    identity
        .and_then(|id| id.id().ok())
        .and_then(|s| Uuid::parse_str(&s).ok())
}

#[derive(Debug, Deserialize)]
pub struct MuteRequest {
    pub tournament_id: String,
}

#[post("/api/v1/users/me/tournament-chat-mutes")]
pub async fn add_mute(
    identity: Option<Identity>,
    body: web::Json<MuteRequest>,
    pool: web::Data<DbPool>,
) -> HttpResponse {
    let user_id = match user_id_from_identity(identity.as_ref()) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "Not authenticated" }
            }));
        }
    };

    let mut conn = match get_conn(pool.get_ref()).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("tournament mute add: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    match mute_tournament_chat(&mut conn, user_id, body.tournament_id.trim()).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "data": { "tournament_id": body.tournament_id }
        })),
        Err(e) => {
            if matches!(e, db_lib::db_error::DbError::NotFound { .. }) {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "Tournament not found" }
                }));
            }
            log::error!("tournament mute add: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Failed to mute tournament chat" }
            }))
        }
    }
}

#[delete("/api/v1/users/me/tournament-chat-mutes/{tournament_id}")]
pub async fn remove_mute(
    identity: Option<Identity>,
    path: web::Path<String>,
    pool: web::Data<DbPool>,
) -> HttpResponse {
    let user_id = match user_id_from_identity(identity.as_ref()) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "Not authenticated" }
            }));
        }
    };
    let tournament_id = path.into_inner();

    let mut conn = match get_conn(pool.get_ref()).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("tournament mute remove: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    if unmute_tournament_chat(&mut conn, user_id, tournament_id.trim()).await.is_err() {
        log::error!("tournament mute remove: unmute failed");
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "data": { "message": "Failed to unmute tournament chat" }
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": { "tournament_id": tournament_id }
    }))
}
