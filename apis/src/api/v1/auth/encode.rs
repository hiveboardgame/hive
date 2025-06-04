use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Bot {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub exp: i64,
}

pub fn jwt_encode(bot: Bot, key: &EncodingKey) -> Result<String> {
    let token = encode(
        &Header::default(),
        &Claims {
            iss: String::from("hivegame.com"),
            sub: bot.email,
            exp: (Utc::now() + Duration::minutes(100)).timestamp(),
        },
        key,
    )?;
    Ok(token)
}
