use base64::Engine;
use sha2::{Digest, Sha256};

const ENGINE: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Returns `(plaintext, token_hash)`. The plaintext goes in the email link; only
/// the hash is persisted in `email_tokens`.
pub fn generate_token() -> (String, String) {
    let bytes: [u8; 32] = rand::random();
    let plaintext = ENGINE.encode(bytes);
    let hash = hash_token(&plaintext);
    (plaintext, hash)
}

pub fn hash_token(plaintext: &str) -> String {
    ENGINE.encode(Sha256::digest(plaintext.as_bytes()))
}
