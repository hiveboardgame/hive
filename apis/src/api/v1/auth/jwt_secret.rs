use jsonwebtoken::{DecodingKey, EncodingKey};

pub struct JwtSecret {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl JwtSecret {
    pub fn new(key: String) -> Self {
        Self {
            encoding: EncodingKey::from_secret(key.as_bytes()),
            decoding: DecodingKey::from_secret(key.as_bytes()),
        }
    }
}
