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
    pub user_id: Uuid,
    pub username: String,
    pub msg: String,
    pub game_id: String,
}
