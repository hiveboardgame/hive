use actix::prelude::*;
use uuid::Uuid;

use crate::common::server_result::MessageDestination;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub addr: Recipient<WsMessage>,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ClientActorMessage {
    pub destination: MessageDestination,
    pub from: Uuid,
    pub serialized: String, // the serialized message
}

impl ClientActorMessage {
    pub fn new(from: Uuid, destination: MessageDestination, serialized: &str) -> Self {
        Self {
            from,
            destination,
            serialized: serialized.to_owned(),
        }
    }
}
