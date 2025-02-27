use super::encode::{jwt_encode, Bot};
use super::jwt_secret::JwtSecret;
use actix_web::post;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use db_lib::get_conn;
use db_lib::{models::User, DbPool};
use serde_json::json;

#[post("/api/v1/auth/token")]
pub async fn get_token(
    Json(bot): Json<Bot>,
    pool: Data<DbPool>,
    jwt_secret: Data<JwtSecret>,
) -> HttpResponse {
    let mut conn = match get_conn(&pool).await {
        Ok(conn) => conn,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
            "success": false,
            "data": {
              "message": "Internal server error",
            }}))
        }
    };
    let user = match User::find_by_email(&bot.email, &mut conn).await {
        Ok(user) => user,
        Err(e) => {
            return HttpResponse::NotFound().json(json!({
              "success": false,
              "data": {
                "message": "No user found by that email address",
              }
            }))
        }
    };
    let argon2 = Argon2::default();
    let stored_pw = match PasswordHash::new(&user.password) {
        Ok(pw) => pw,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
            "success": false,
            "data": {
              "message": "Internal server error",
            }}))
        }
    };

    if argon2
        .verify_password(bot.password.as_bytes(), &stored_pw)
        .is_err()
    {
        return HttpResponse::BadRequest().json(json!({
          "success": false,
          "data": {
            "message": "Password does not match"
          }
        }));
    }

    match jwt_encode(bot, &jwt_secret.encoding) {
        Ok(token) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "token": token
          }
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
        "success": false,
        "data": {
          "message": "Internal server error",
        }})),
    }
}
