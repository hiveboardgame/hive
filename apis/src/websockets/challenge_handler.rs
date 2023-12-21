use crate::common::server_result::InternalServerMessage;
use anyhow::Result;

pub struct ChallengeHandler {}

impl ChallengeHandler {
    pub async fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = Vec::new();
        Ok(messages)
    }
}
