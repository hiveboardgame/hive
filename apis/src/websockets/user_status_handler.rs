use anyhow::Result;

use super::internal_server_message::InternalServerMessage;

pub struct UserStatusHandler {}

impl UserStatusHandler {
    pub async fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = Vec::new();
        Ok(messages)
    }
}
