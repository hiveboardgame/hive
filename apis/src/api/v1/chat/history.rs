//! GET /api/v1/chat/channel — fetch chat history for a channel.
//! Query: channel_type, channel_id, limit (default 50), before_id (optional, for pagination).
//! Requires session (Identity).

use actix_identity::Identity;
use actix_web::{get, web::Data, web::Query, HttpResponse};
use db_lib::{
    get_conn,
    helpers::{
        can_user_access_chat_channel, get_blocked_user_ids, get_chat_messages_for_channel,
    },
    DbPool,
};
use shared_types::CHANNEL_TYPE_DIRECT;
use serde::Deserialize;
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct ChannelQuery {
    pub channel_type: String,
    pub channel_id: String,
    pub limit: Option<i64>,
    pub before_id: Option<i64>,
}

#[get("/api/v1/chat/channel")]
pub async fn get_channel_history(
    identity: Option<Identity>,
    query: Query<ChannelQuery>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let user_id = match identity.as_ref().and_then(|id| id.id().ok()).and_then(|s| Uuid::parse_str(&s).ok()) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "Not authenticated" }
            }));
        }
    };

    let limit = query
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    let mut conn = match get_conn(pool.get_ref()).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("chat history: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    let allowed = match can_user_access_chat_channel(
        &mut conn,
        user_id,
        &query.channel_type,
        &query.channel_id,
    )
    .await
    {
        Ok(a) => a,
        Err(e) => {
            log::error!("chat history: auth check failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };
    if !allowed {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "data": { "message": "Access denied" }
        }));
    }

    let mut messages = match get_chat_messages_for_channel(
        &mut conn,
        &query.channel_type,
        &query.channel_id,
        limit,
        query.before_id,
    )
    .await
    {
        Ok(m) => m,
        Err(e) => {
            log::error!("chat history: query failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Failed to load messages" }
            }));
        }
    };

    if query.channel_type == CHANNEL_TYPE_DIRECT {
        let blocked = get_blocked_user_ids(&mut conn, user_id)
            .await
            .unwrap_or_default();
        let blocked: std::collections::HashSet<Uuid> = blocked.into_iter().collect();
        messages.retain(|m| !blocked.contains(&m.sender_id));
    }

    let data: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "channel_type": m.channel_type,
                "channel_id": m.channel_id,
                "sender_id": m.sender_id,
                "username": m.username,
                "body": m.body,
                "turn": m.turn,
                "created_at": m.created_at.to_rfc3339(),
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": data
    }))
}
