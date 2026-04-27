use actix::prelude::*;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::common::ServerMessage;
use super::telemetry::SendOutcome;

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

#[derive(Clone, Debug)]
pub struct SocketTx {
    pub socket_id: Uuid,
    pub tx: mpsc::Sender<Vec<u8>>,
}

impl SocketTx {
    /// Enqueue bytes for delivery. Returns false if the buffer is full or the
    /// receiver is gone — caller should drop this socket from its sessions map.
    pub fn try_send(&self, bytes: Vec<u8>) -> bool {
        self.tx.try_send(bytes).is_ok()
    }

    pub fn try_send_classified(&self, bytes: Vec<u8>) -> SendOutcome {
        match self.tx.try_send(bytes) {
            Ok(_) => SendOutcome::Ok,
            Err(mpsc::error::TrySendError::Full(_)) => SendOutcome::Full,
            Err(mpsc::error::TrySendError::Closed(_)) => SendOutcome::Closed,
        }
    }

    pub fn capacity_used(&self) -> usize {
        128usize.saturating_sub(self.tx.capacity())
    }
}

#[derive(Debug, Clone)]
pub enum MessageDestination {
    Direct(SocketTx),
    User(Uuid),
    Game(GameId),
    GameSpectators(GameId, Uuid, Uuid),
    Global,
    Tournament(TournamentId),
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Connect {
    pub socket: SocketTx,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub socket_id: Uuid,
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
    pub serialized: Vec<u8>,
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
