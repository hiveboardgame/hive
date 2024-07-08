use super::messages::WsMessage;
use crate::common::ServerMessage;
use shared_types::{GameId, TournamentId};
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
    Game(GameId),                        // to everyone in the game
    GameSpectators(GameId, Uuid, Uuid), // to everyone in game excluding players, nanoid, white_id, black_id
    Global,                             // to everyone online
    Tournament(TournamentId),           // to everyone that joined the tournament
}
