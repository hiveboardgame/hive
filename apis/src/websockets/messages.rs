use actix::prelude::*;
use uuid::Uuid;

use super::internal_server_message::MessageDestination;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct WsMessage(pub Vec<u8>);

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
pub struct GameHB {}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Ping {}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ClientActorMessage {
    pub destination: MessageDestination,
    pub from: Option<Uuid>,
    pub serialized: Vec<u8>, // the serialized message
}

impl ClientActorMessage {
    pub fn new(from: Option<Uuid>, destination: MessageDestination, serialized: &Vec<u8>) -> Self {
        Self {
            from,
            destination,
            serialized: serialized.to_owned(),
        }
    }
}
