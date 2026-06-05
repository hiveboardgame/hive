use super::{
    encode::{jwt_encode, Bot},
    jwt_secret::JwtSecret,
};
use crate::functions::auth::password::verify_password;
use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse,
};
use anyhow::{anyhow, Result};
use db_lib::{get_conn, models::User, DbPool};
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

    let user = User::find_for_login(&bot.email, &mut conn).await?;

    if !user.bot {
        return Err(anyhow!("Not a bot"));
    }
    verify_password(&bot.password, &user.password).map_err(|e| anyhow!(e.to_string()))?;
    jwt_encode(bot, &jwt_secret.encoding)
}
