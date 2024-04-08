use super::messages::WsMessage;
use crate::common::server_result::ServerMessage;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct InternalServerMessage {
    pub destination: MessageDestination,
    pub message: ServerMessage,
}

#[derive(Debug, Clone)]
pub enum MessageDestination {
    Direct(actix::Recipient<WsMessage>),
    User(Uuid),   // to a user
    Game(String), // to everyone in the game
    Global,       // to everyone online
}
