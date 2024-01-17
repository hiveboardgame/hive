use crate::common::server_result::{InternalServerMessage, MessageDestination, ServerMessage};
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
            destination: MessageDestination::Direct(self.user_id),
            message: ServerMessage::Pong {
                ping_sent: self.sent,
                pong_sent: Utc::now(),
            },
        }]
    }
}
