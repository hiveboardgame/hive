use super::{bearer::resolve_bearer_user, jwt_secret::JwtSecret};
use actix_web::{get, web::Data, HttpRequest, HttpResponse};
use db_lib::DbPool;
use serde_json::json;

/// Lets a personal-access-token holder (not just bots, unlike `/auth/id`)
/// learn their own `user_id`/`username`.
#[get("/api/v1/auth/whoami")]
pub async fn get_whoami(
    req: HttpRequest,
    pool: Data<DbPool>,
    jwt_secret: Data<JwtSecret>,
) -> HttpResponse {
    match resolve_bearer_user(&req, &pool, &jwt_secret).await {
        Some(user) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": {
                "user_id": user.id,
                "username": user.username,
            }
        })),
        None => HttpResponse::Unauthorized().json(json!({
            "success": false,
            "data": {
                "message": "invalid or missing bearer token",
            }
        })),
    }
}
