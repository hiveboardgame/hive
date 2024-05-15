use super::messages::WsMessage;
use crate::common::ServerMessage;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InternalServerMessage {
    pub destination: MessageDestination,
    pub message: ServerMessage,
}

#[derive(Debug, Clone)]
pub enum MessageDestination {
    Direct(actix::Recipient<WsMessage>), // to non logged in user
    User(Uuid),                          // to a user
    Game(String),                        // to everyone in the game
    GameSpectators(String, Uuid, Uuid), // to everyone in game excluding players, nanoid, white_id, black_id
    Global,                             // to everyone online
    Tournament(String),                 // to everyone that joined the tournament
}
