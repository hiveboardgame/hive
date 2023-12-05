use actix::prelude::*;
use uuid::Uuid;

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
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ClientActorMessage {
    pub game_id: String, // game room to send the message to other users (if needed)
    pub serialized: String, // the serialized message
    pub user_id: Uuid,   // needed to find the websocket to send the message over
}

impl ClientActorMessage {
    pub fn new(game_id: &str, serialized: &str, user_id: Uuid) -> Self {
        Self {
            game_id: game_id.to_owned(),
            serialized: serialized.to_owned(),
            user_id,
        }
    }
}
