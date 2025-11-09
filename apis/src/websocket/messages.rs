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
    User(Uuid),                         // to a user
    Game(GameId),                       // to everyone in the game
    GameSpectators(GameId, Uuid, Uuid), // to everyone in game excluding players, nanoid, white_id, black_id
    Global,                             // to everyone online
    Tournament(TournamentId),           // to everyone that joined the tournament
}
