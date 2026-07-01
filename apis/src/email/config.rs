use lettermint_rs::reqwest::LettermintClient;
use std::sync::Arc;

const DEFAULT_BASE_URL: &str = if cfg!(debug_assertions) {
    "http://127.0.0.1:3000"
} else {
    "https://hivegame.com"
};

#[derive(Clone)]
pub struct EmailConfig {
    pub base_url: String,
    pub client: Option<Arc<LettermintClient>>,
}

impl EmailConfig {
    pub fn from_env() -> EmailConfig {
        let base_url =
            std::env::var("EMAIL_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        EmailConfig {
            base_url,
            client: build_client(),
        }
    }
}

fn build_client() -> Option<Arc<LettermintClient>> {
    if cfg!(debug_assertions) {
        return None;
    }
    std::env::var("LETTERMINT_API_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .map(|key| Arc::new(LettermintClient::builder().api_token(key).build()))
}
