//! Block list API: only the authenticated user (from session) can block/unblock.
//! POST /api/v1/users/me/blocks — block a user (body: { "user_id": "uuid" }).
//! DELETE /api/v1/users/me/blocks/{user_id} — unblock.
//! GET /api/v1/users/me/blocks — list blocked user IDs.

use actix_identity::Identity;
use actix_web::{delete, get, post, web, HttpResponse};
use db_lib::{
    get_conn,
    helpers::{block_user, get_blocked_user_ids, unblock_user},
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
pub struct BlockRequest {
    pub user_id: Uuid,
}

#[post("/api/v1/users/me/blocks")]
pub async fn add_block(
    identity: Option<Identity>,
    body: web::Json<BlockRequest>,
    pool: web::Data<DbPool>,
) -> HttpResponse {
    let blocker_id = match user_id_from_identity(identity.as_ref()) {
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
            log::error!("blocks add: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    match block_user(&mut conn, blocker_id, body.user_id).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "data": { "blocked_id": body.user_id }
        })),
        Err(e) => {
            if let db_lib::db_error::DbError::InvalidInput { .. } = e {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "Cannot block yourself" }
                }));
            }
            log::error!("blocks add: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Failed to block user" }
            }))
        }
    }
}

#[delete("/api/v1/users/me/blocks/{user_id}")]
pub async fn remove_block(
    identity: Option<Identity>,
    path: web::Path<Uuid>,
    pool: web::Data<DbPool>,
) -> HttpResponse {
    let blocker_id = match user_id_from_identity(identity.as_ref()) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "Not authenticated" }
            }));
        }
    };
    let blocked_id = path.into_inner();

    let mut conn = match get_conn(pool.get_ref()).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("blocks remove: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    if unblock_user(&mut conn, blocker_id, blocked_id).await.is_err() {
        log::error!("blocks remove: unblock failed");
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "data": { "message": "Failed to unblock user" }
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": { "unblocked_id": blocked_id }
    }))
}

#[get("/api/v1/users/me/blocks")]
pub async fn list_blocks(identity: Option<Identity>, pool: web::Data<DbPool>) -> HttpResponse {
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
            log::error!("blocks list: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    let blocked_ids = match get_blocked_user_ids(&mut conn, user_id).await {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("blocks list: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Failed to list blocks" }
            }));
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": { "blocked_user_ids": blocked_ids }
    }))
}
