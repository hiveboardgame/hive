use crate::api::v1::auth::auth::Auth;
use crate::responses::UserResponse;
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use anyhow::Result;
use db_lib::{get_conn, DbPool};
use serde_json::json;
use uuid::Uuid;

#[get("/api/v1/bot/user/{id}")]
pub async fn api_get_user(id: Path<Uuid>, Auth(email): Auth, pool: Data<DbPool>) -> HttpResponse {
    let id = id.into_inner();
    match get_user(id, pool).await {
        Ok(user) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "bot": email,
            "user": user,
          }
        })),
        Err(e) => HttpResponse::Ok().json(json!({
          "success": false,
          "data": {
            "error": e.to_string(),
          }
        })),
    }
}

async fn get_user(id: Uuid, pool: Data<DbPool>) -> Result<UserResponse> {
    let mut conn = get_conn(&pool).await?;
    UserResponse::from_uuid(&id, &mut conn).await
}
