use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// Encode a JWT for a regular user (HiveGame mobile app, future native clients).
/// `sub` is the user's UUID — `identity::uuid()` matches against this on the
/// backend. Longer expiry than bot tokens (30 days) so mobile users don't
/// re-login constantly; JWTs are stateless so revocation waits for expiry.
pub fn jwt_encode_user_id(user_id: Uuid, key: &EncodingKey) -> Result<String> {
    let token = encode(
        &Header::default(),
        &Claims {
            iss: String::from("hivegame.com"),
            sub: user_id.to_string(),
            exp: (Utc::now() + Duration::days(30)).timestamp(),
        },
        key,
    )?;
    Ok(token)
}
