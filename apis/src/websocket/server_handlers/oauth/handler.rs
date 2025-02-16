use crate::{
    common::ServerMessage,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
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
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            //.header(USER_AGENT, "rust-web-api-client") // gh api requires a user-agent header
            .send()
            .await?;

        let json_str = response.text().await?;
        let json: Value = serde_json::from_str(&json_str)?;
        println!("Body: {}", json_str);
        let link = json["url"].as_str().unwrap();

        println!("Link {:?}", link);
        let message = InternalServerMessage {
            destination: MessageDestination::User(self.uuid),
            message: ServerMessage::RedirectLink(link.to_string()),
        };
        messages.push(message);
        Ok(messages)
    }
}
