use crate::{
    common::server_result::ServerMessage,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct PingHandler {
    user_id: Uuid,
    sent: DateTime<Utc>,
}

impl PingHandler {
    pub fn new(user_id: Uuid, sent: DateTime<Utc>) -> Self {
        Self { user_id, sent }
    }

    pub fn handle(&self) -> Vec<InternalServerMessage> {
        vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Pong {
                ping_sent: self.sent,
                pong_sent: Utc::now(),
            },
        }]
    }
}
