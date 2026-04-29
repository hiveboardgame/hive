use crate::websocket::messages::{InternalServerMessage, MessageDestination};
use anyhow::Result;
use serde_json::Value;
use uuid::Uuid;

pub struct OauthHandler {
    uuid: Uuid,
}

impl OauthHandler {
    pub fn new(uuid: Uuid) -> Self {
        Self { uuid }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        let url = format!("http://localhost:8080/oauth/new/{}", self.uuid);
        let client = match reqwest::Client::builder().build() {
            Ok(client) => client,
            Err(e) => {
                println!("Failed to create Discord OAuth client: {e}");
                return Ok(messages);
            }
        };
        let response = match client.post(url).send().await {
            Ok(response) => response,
            Err(e) => {
                println!("Failed to start Discord OAuth flow: {e}");
                return Ok(messages);
            }
        };

        let json_str = match response.text().await {
            Ok(json_str) => json_str,
            Err(e) => {
                println!("Failed to read Discord OAuth response: {e}");
                return Ok(messages);
            }
        };
        let json: Value = match serde_json::from_str(&json_str) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse Discord OAuth response: {e}");
                return Ok(messages);
            }
        };
        if let Some(url) = json.get("url") {
            let url = url.to_string().replace("\"", "");

            let message = InternalServerMessage {
                destination: MessageDestination::User(self.uuid),
                message: crate::common::ServerMessage::RedirectLink(url.to_string()),
            };
            messages.push(message);
        }
        Ok(messages)
    }
}
