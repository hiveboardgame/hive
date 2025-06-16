use super::jwt_secret::JwtSecret;
use actix_web::{
    dev::Payload, error::InternalError, http::header, FromRequest, HttpRequest, HttpResponse,
};
use serde_json::json;
use std::future::{ready, Ready};
pub struct Auth(pub String);
use actix_web::web::Data;

impl FromRequest for Auth {
    type Error = InternalError<String>;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let access_token = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|str| str.split(" ").nth(1));
        let jwt_secret = req.app_data::<Data<JwtSecret>>().unwrap();

        match access_token {
            Some(token) => match super::decode::jwt_decode(token, &jwt_secret.decoding) {
                Ok(email) => ready(Ok(Auth(email))),

                Err(e) => ready(Err(InternalError::from_response(
                    e.to_string(),
                    HttpResponse::Unauthorized().json(json!({
                      "success": false,
                      "data": {
                        "message": e.to_string(),
                      }
                    })),
                ))),
            },

            None => ready(Err(InternalError::from_response(
                String::from("No token provided"),
                HttpResponse::Unauthorized().json(json!({
                  "success": false,
                  "data": {
                    "message": "No token provided"
                  }
                })),
            ))),
        }
    }
}
