use super::Auth;
use actix_web::get;
use actix_web::HttpResponse;
use serde_json::json;

#[get("/api/v1/auth/id")]
pub async fn get_identity(Auth(bot): Auth) -> HttpResponse {
    HttpResponse::Ok().json(json!({
      "success": true,
      "data": {
        "bot": bot.email,
        "user_id": bot.id
      }
    }))
}
