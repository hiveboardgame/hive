use bytes::Bytes;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
};
use db_lib::models::Game;

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

#[derive(Debug, Clone, Copy)]
pub enum GameSpectatorAudience {
    GameViewers,
    SpectatorChat { include_players: bool },
}

#[derive(Debug, Clone, Copy)]
pub enum TournamentAudience {
    Updates,
    ScheduleViewers,
    Chat { sender_id: Uuid },
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
pub(crate) struct TerminalGameRetry {
    pub(crate) game: Game,
    pub(crate) reaction: GameReaction,
    pub(crate) actor_id: Uuid,
    pub(crate) actor_username: Option<String>,
    pub(crate) publish_tv: bool,
}

/// A `GameUpdate::Reaction` event that needs to fan out to both players and
/// every spectator. Carrying the unserialized payload lets the dispatcher
/// (`WsHub::dispatch_reaction`) msgpack-encode it **once** and `Bytes::clone`
/// the result across the three destinations — saving two redundant
/// serializations of a non-trivial payload per turn/control.
///
/// Use this in handlers that return a `HandlerOutput` so every committed Game
/// publication shares the same outer dispatch path.
#[derive(Debug, Clone)]
pub struct Reaction {
    pub game_id: GameId,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub gar: GameActionResponse,
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

/// Aggregated handler return: messages, reaction events
/// (dispatched with a single shared `Bytes`), plus post-dispatch
/// finalizations. `From<Vec<InternalServerMessage>>` lets handlers that
/// never finalize a game and never emit reactions keep their existing
/// return shape.
#[derive(Debug, Default)]
pub struct HandlerOutput {
    pub messages: Vec<InternalServerMessage>,
    pub reactions: Vec<Reaction>,
    pub tv_updates: Vec<TvUpdate>,
    pub finalize_games: Vec<GameFinalize>,
    pub(crate) terminal_game_retries: Vec<TerminalGameRetry>,
    pub request_error: Option<anyhow::Error>,
}

impl HandlerOutput {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn append(&mut self, mut other: Self) {
        self.messages.append(&mut other.messages);
        self.reactions.append(&mut other.reactions);
        self.tv_updates.append(&mut other.tv_updates);
        self.finalize_games.append(&mut other.finalize_games);
        self.terminal_game_retries
            .append(&mut other.terminal_game_retries);
        if self.request_error.is_none() {
            self.request_error = other.request_error;
        }
    }
}

impl From<Vec<InternalServerMessage>> for HandlerOutput {
    fn from(messages: Vec<InternalServerMessage>) -> Self {
        Self {
            messages,
            reactions: Vec::new(),
            tv_updates: Vec::new(),
            finalize_games: Vec::new(),
            terminal_game_retries: Vec::new(),
            request_error: None,
        }
    }
}

#[derive(Debug)]
pub struct TvUpdate {
    pub game_id: GameId,
    pub game: GameResponse,
    pub final_state: bool,
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
    GameSpectators {
        game_id: GameId,
        white_id: Uuid,
        black_id: Uuid,
        audience: GameSpectatorAudience,
    },
    Global,
    Tournament {
        tournament_id: TournamentId,
        audience: TournamentAudience,
    },
}
