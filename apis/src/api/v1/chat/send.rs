//! POST /api/v1/chat/send — send a chat message (persisted to DB).
//! Requires session (Identity). Body: { channel_type, channel_id, body, turn? }.

use crate::websocket::server_handlers::chat::persist::PersistableChatMessage;
use actix_identity::Identity;
use actix_web::{post, web::Data, web::Json, HttpResponse};
use db_lib::{
    get_conn,
    helpers::{insert_chat_message, is_blocked},
    models::{Game, Tournament, User},
    DbPool,
};
use serde::Deserialize;
use shared_types::{
    canonical_dm_channel_id, GameId, CHANNEL_TYPE_DIRECT, CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_GAME_SPECTATORS, CHANNEL_TYPE_GLOBAL, CHANNEL_TYPE_TOURNAMENT_LOBBY,
};
use uuid::Uuid;

const MAX_MESSAGE_LENGTH: usize = 1000;
const VALID_CHANNEL_TYPES: [&str; 5] = [
    shared_types::CHANNEL_TYPE_GAME_PLAYERS,
    shared_types::CHANNEL_TYPE_GAME_SPECTATORS,
    shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY,
    CHANNEL_TYPE_DIRECT,
    CHANNEL_TYPE_GLOBAL,
];

#[derive(Debug, Deserialize)]
pub struct SendChatRequest {
    pub channel_type: String,
    pub channel_id: String,
    pub body: String,
    pub turn: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
pub struct SendChatResponse {
    pub id: i64,
    pub channel_type: String,
    pub channel_id: String,
    pub sender_id: Uuid,
    pub username: String,
    pub body: String,
    pub turn: Option<i32>,
    pub created_at: String,
}

#[post("/api/v1/chat/send")]
pub async fn send_chat(
    identity: Option<Identity>,
    body: Json<SendChatRequest>,
    pool: Data<DbPool>,
) -> HttpResponse {
    let identity = match identity {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "Not authenticated" }
            }));
        }
    };

    let user_id = match identity
        .id()
        .ok()
        .and_then(|s| Uuid::parse_str(&s).ok())
    {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "Invalid session" }
            }));
        }
    };

    let mut conn = match get_conn(pool.get_ref()).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("chat send: db connection failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Database error" }
            }));
        }
    };

    let user = match User::find_by_uuid(&user_id, &mut conn).await {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "data": { "message": "User not found" }
            }));
        }
    };

    if !VALID_CHANNEL_TYPES.contains(&body.channel_type.as_str()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "data": { "message": "Invalid channel_type" }
        }));
    }

    if body.channel_type == CHANNEL_TYPE_GLOBAL && !user.admin {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "data": { "message": "Global chat requires admin" }
        }));
    }

    if body.channel_type == CHANNEL_TYPE_TOURNAMENT_LOBBY {
        let tournament = match Tournament::from_nanoid(&body.channel_id, &mut conn).await {
            Ok(t) => t,
            Err(_) => {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "Tournament not found" }
                }))
            }
        };
        let is_player = tournament
            .players(&mut conn)
            .await
            .map(|p| p.iter().any(|u| u.id == user_id))
            .unwrap_or(false);
        let is_organizer = tournament
            .organizers(&mut conn)
            .await
            .map(|o| o.iter().any(|u| u.id == user_id))
            .unwrap_or(false);
        if !is_player && !is_organizer {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "success": false,
                "data": { "message": "Only tournament participants and organizers can send messages" }
            }));
        }
    }

    if body.channel_type == CHANNEL_TYPE_GAME_PLAYERS {
        let game = match Game::find_by_game_id(&GameId(body.channel_id.clone()), &mut conn).await {
            Ok(g) => g,
            Err(_) => {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "Game not found" }
                }))
            }
        };
        if user_id != game.white_id && user_id != game.black_id {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "success": false,
                "data": { "message": "Only players can send to players chat" }
            }));
        }
    }

    if body.channel_type == CHANNEL_TYPE_GAME_SPECTATORS {
        let game = match Game::find_by_game_id(&GameId(body.channel_id.clone()), &mut conn).await {
            Ok(g) => g,
            Err(_) => {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "Game not found" }
                }))
            }
        };
        let is_player = user_id == game.white_id || user_id == game.black_id;
        if is_player && !game.finished {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "success": false,
                "data": { "message": "Players cannot send to spectators chat while the game is ongoing" }
            }));
        }
    }

    let mut channel_id = body.channel_id.clone();
    if body.channel_type == CHANNEL_TYPE_DIRECT {
        // Always validate sender is part of the DM channel and canonicalize.
        // This prevents user A from injecting messages into user B <-> C's DM.
        let other_id = if channel_id.contains("::") {
            // Preformatted channel_id: extract the "other" user and verify sender is included
            let parts: Vec<&str> = channel_id.split("::").collect();
            if parts.len() != 2 {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "Invalid DM channel_id format" }
                }));
            }
            let a = match Uuid::parse_str(parts[0]) {
                Ok(u) => u,
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "success": false,
                        "data": { "message": "Invalid UUID in channel_id" }
                    }));
                }
            };
            let b = match Uuid::parse_str(parts[1]) {
                Ok(u) => u,
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "success": false,
                        "data": { "message": "Invalid UUID in channel_id" }
                    }));
                }
            };
            if a == user_id {
                b
            } else if b == user_id {
                a
            } else {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "success": false,
                    "data": { "message": "You are not a participant in this DM" }
                }));
            }
        } else {
            // Single UUID: treat as the other user's ID
            match Uuid::parse_str(&channel_id) {
                Ok(u) => u,
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "success": false,
                        "data": { "message": "Invalid channel_id for DM" }
                    }));
                }
            }
        };
        channel_id = canonical_dm_channel_id(user_id, other_id);
        // Recipient has blocked sender: do not deliver or persist.
        if is_blocked(&mut conn, other_id, user_id).await.unwrap_or(false) {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "success": false,
                "data": { "message": "You cannot send messages to this user" }
            }));
        }
    }

    let mut body_text = body.body.clone();
    if body_text.len() > MAX_MESSAGE_LENGTH {
        body_text.truncate(MAX_MESSAGE_LENGTH);
    }

    let persistable = PersistableChatMessage::from_parts(
        body.channel_type.clone(),
        channel_id.clone(),
        user_id,
        user.username.clone(),
        body_text.clone(),
        body.turn,
    );

    let created = match insert_chat_message(&mut conn, persistable.as_new()).await {
        Ok(m) => m,
        Err(e) => {
            log::error!("chat send: insert failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "data": { "message": "Failed to save message" }
            }));
        }
    };

    HttpResponse::Created().json(serde_json::json!({
        "success": true,
        "data": {
            "id": created.id,
            "channel_type": created.channel_type,
            "channel_id": created.channel_id,
            "sender_id": created.sender_id,
            "username": created.username,
            "body": created.body,
            "turn": created.turn,
            "created_at": created.created_at.to_rfc3339(),
        }
    }))
}
