use super::encode::Claims;
use anyhow::Result;
use jsonwebtoken::{decode, DecodingKey, Validation};

pub fn jwt_decode(token: &str, key: &DecodingKey) -> Result<String> {
    let data = decode::<Claims>(token, key, &Validation::default())?;
    Ok(data.claims.sub)
}
