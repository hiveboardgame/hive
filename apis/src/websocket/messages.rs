use bytes::Bytes;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::common::{GameActionResponse, GameUpdate, ServerMessage};

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

/// Per-game finalization request returned from a handler. The dispatcher runs
/// finalization *after* sending all messages, so the handler's `Game(game_id)`
/// fanout still reaches subscribers.
#[derive(Debug, Clone)]
pub struct GameFinalize {
    pub game_id: GameId,
    pub white_id: Uuid,
    pub black_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum GameSubscription {
    Fanout(GameId),
    Heartbeat(GameId),
}

/// A `GameUpdate::Reaction` event that needs to fan out to both players and
/// every spectator. Carrying the unserialized payload lets the dispatcher
/// (`WsHub::dispatch_reaction`) msgpack-encode it **once** and `Bytes::clone`
/// the result across the three destinations — saving two redundant
/// serializations of a non-trivial payload per turn/control.
///
/// Use this in handlers that return a `HandlerOutput`. Paths that
/// build a flat `Vec<InternalServerMessage>` (bot API, periodic jobs) can
/// still call `Reaction::into_messages` to get the legacy three-message
/// expansion.
#[derive(Debug, Clone)]
pub struct Reaction {
    pub game_id: GameId,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub gar: GameActionResponse,
}

impl Reaction {
    /// Expand into three `InternalServerMessage`s for callers that don't
    /// go through `HandlerOutput.reactions` and so can't take advantage of
    /// the single serialization in `WsHub::dispatch_reaction`. Each call
    /// site here pays for two payload clones plus two extra msgpack
    /// serializations — fine for low-volume HTTP/cron paths.
    pub fn into_messages(self) -> Vec<InternalServerMessage> {
        let payload = ServerMessage::Game(Box::new(GameUpdate::Reaction(self.gar)));
        vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.white_id),
                message: payload.clone(),
            },
            InternalServerMessage {
                destination: MessageDestination::User(self.black_id),
                message: payload.clone(),
            },
            InternalServerMessage {
                destination: MessageDestination::GameSpectators(
                    self.game_id,
                    self.white_id,
                    self.black_id,
                ),
                message: payload,
            },
        ]
    }
}

/// Legacy entry point: build the three-message expansion of a reaction.
/// Hot WS handlers should push to `HandlerOutput.reactions` instead and
/// rely on `WsHub::dispatch_reaction` to serialize once. Retained for the
/// bot API + tournament_start dispatch paths.
pub fn reaction_messages(
    game_id: GameId,
    white_id: Uuid,
    black_id: Uuid,
    gar: GameActionResponse,
) -> Vec<InternalServerMessage> {
    Reaction {
        game_id,
        white_id,
        black_id,
        gar,
    }
    .into_messages()
}

impl GameFinalize {
    pub fn own_game_removed_messages(&self) -> Vec<InternalServerMessage> {
        [self.white_id, self.black_id]
            .into_iter()
            .map(|user_id| InternalServerMessage {
                destination: MessageDestination::User(user_id),
                message: ServerMessage::Game(Box::new(GameUpdate::OwnGameRemoved(
                    self.game_id.clone(),
                ))),
            })
            .collect()
    }
}

/// Aggregated handler return: pre-encoded messages, reaction events
/// (dispatched with a single shared `Bytes`), plus post-dispatch
/// finalizations. `From<Vec<InternalServerMessage>>` lets handlers that
/// never finalize a game and never emit reactions keep their existing
/// return shape.
#[derive(Debug, Default)]
pub struct HandlerOutput {
    pub messages: Vec<InternalServerMessage>,
    pub reactions: Vec<Reaction>,
    pub finalize_games: Vec<GameFinalize>,
    pub subscriptions: Vec<GameSubscription>,
}

impl HandlerOutput {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl From<Vec<InternalServerMessage>> for HandlerOutput {
    fn from(messages: Vec<InternalServerMessage>) -> Self {
        Self {
            messages,
            reactions: Vec::new(),
            finalize_games: Vec::new(),
            subscriptions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SocketTx {
    pub socket_id: Uuid,
    pub tx: mpsc::Sender<Bytes>,
}

#[derive(Debug, Clone)]
pub enum MessageDestination {
    Direct(SocketTx),
    User(Uuid),
    Game(GameId),
    GameSpectators(GameId, Uuid, Uuid),
    Global,
    /// Tournament members fanout, with an optional user to echo exactly once.
    Tournament(TournamentId, Option<Uuid>),
}
