use super::{
    encode::{jwt_encode, PersonalTokenRequest},
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

#[post("/api/v1/auth/personal_token")]
pub async fn get_personal_token(
    Json(request): Json<PersonalTokenRequest>,
    pool: Data<DbPool>,
    jwt_secret: Data<JwtSecret>,
) -> HttpResponse {
    match get_personal_token_helper(request, jwt_secret, pool).await {
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

async fn get_personal_token_helper(
    request: PersonalTokenRequest,
    jwt_secret: Data<JwtSecret>,
    pool: Data<DbPool>,
) -> Result<String> {
    let mut conn = get_conn(&pool).await?;

    let user = User::find_for_login(&request.email, &mut conn).await?;

    verify_password(&request.password, &user.password).map_err(|e| anyhow!(e.to_string()))?;
    jwt_encode(request.email, &jwt_secret.encoding)
}
