use super::encode::{jwt_encode, Bot};
use super::jwt_secret::JwtSecret;
use actix_web::post;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use anyhow::{anyhow, Result};
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
    match get_token_helper(bot, jwt_secret, pool).await {
        Ok(token) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": {
                "token": token
            }
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "data": {
                "message": e.to_string(),
            }
        })),
    }
}

async fn get_token_helper(
    bot: Bot,
    jwt_secret: Data<JwtSecret>,
    pool: Data<DbPool>,
) -> Result<String> {
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_email(&bot.email, &mut conn).await?;
    if !user.bot {
        return Err(anyhow!("Not a bot"));
    }
    let argon2 = Argon2::default();
    let stored_pw = PasswordHash::new(&user.password).map_err(|e| anyhow!(e.to_string()))?;

    if argon2
        .verify_password(bot.password.as_bytes(), &stored_pw)
        .is_err()
    {
        return Err(anyhow!("Password does not match"));
    }
    jwt_encode(bot, &jwt_secret.encoding)
}
