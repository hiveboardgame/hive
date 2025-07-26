use super::jwt_secret::JwtSecret;
use actix_web::{
    dev::Payload, error::InternalError, http::header, FromRequest, HttpRequest, HttpResponse,
};
use db_lib::{get_conn, models::User, DbPool};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
pub struct Auth(pub User);
use actix_web::web::Data;

fn create_error_response(status: u16, message: &str) -> InternalError<String> {
    let mut response = match status {
        401 => HttpResponse::Unauthorized(),
        400 => HttpResponse::BadRequest(),
        _ => HttpResponse::InternalServerError(),
    };

    InternalError::from_response(
        message.to_string(),
        response.json(json!({
          "success": false,
          "data": {
            "message": message
          }
        })),
    )
}

impl FromRequest for Auth {
    type Error = InternalError<String>;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let access_token = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|str| str.split(" ").nth(1))
            .map(|s| s.to_string());

        let jwt_secret = req.app_data::<Data<JwtSecret>>().cloned();
        let pool = req.app_data::<Data<DbPool>>().cloned();

        Box::pin(async move {
            let token =
                access_token.ok_or_else(|| create_error_response(401, "No token provided"))?;
            let jwt_secret =
                jwt_secret.ok_or_else(|| create_error_response(500, "Internal server error"))?;
            let pool = pool.ok_or_else(|| create_error_response(500, "Internal server error"))?;

            let email = super::decode::jwt_decode(&token, &jwt_secret.decoding)
                .map_err(|e| create_error_response(401, &e.to_string()))?;

            let mut conn = get_conn(&pool)
                .await
                .map_err(|_| create_error_response(500, "Database connection error"))?;

            let user_result = User::find_by_email(&email, &mut conn).await;
            let user = if let Ok(user) = user_result {
                user
            } else {
                User::find_by_username(&email, &mut conn)
                    .await
                    .map_err(|_| create_error_response(400, "User not found"))?
            };

            if !user.bot {
                return Err(create_error_response(401, "Not a bot"));
            }

            Ok(Auth(user))
        })
    }
}
