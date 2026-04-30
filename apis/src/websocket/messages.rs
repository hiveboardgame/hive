use bytes::Bytes;
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
    pub tx: mpsc::Sender<Bytes>,
}

impl SocketTx {
    pub fn try_send_classified(&self, bytes: Bytes) -> SendOutcome {
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
