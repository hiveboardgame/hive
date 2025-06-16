use actix::prelude::*;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use uuid::Uuid;

use crate::common::ServerMessage;

#[derive(thiserror::Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthError {
    #[error("You are not authorized to perform that action")]
    Unauthorized,
}

#[derive(Debug, Clone)]
pub struct InternalServerMessage {
    pub destination: MessageDestination,
    pub message: ServerMessage,
}

#[derive(Debug, Clone)]
pub enum MessageDestination {
    Direct(actix::Recipient<WsMessage>), // to non logged in user
    User(Uuid),                          // to a user
    Game(GameId),                        // to everyone in the game
    GameSpectators(GameId, Uuid, Uuid), // to everyone in game excluding players, nanoid, white_id, black_id
    Global,                             // to everyone online
    Tournament(TournamentId),           // to everyone that joined the tournament
}

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

#[derive(Message, Debug, Clone)]
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
