use bytes::Bytes;
use serde::{Deserialize, Serialize};
use shared_types::{GameId, TournamentId};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::common::{GameActionResponse, GameUpdate, ServerMessage, ServerResult};
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};

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
                destination: MessageDestination::GameSpectators {
                    game_id: self.game_id,
                    white_id: self.white_id,
                    black_id: self.black_id,
                    audience: GameSpectatorAudience::GameViewers,
                },
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

/// Aggregated handler return: messages, reaction events
/// (dispatched with a single shared `Bytes`), plus post-dispatch
/// finalizations. `From<Vec<InternalServerMessage>>` lets handlers that
/// never finalize a game and never emit reactions keep their existing
/// return shape.
#[derive(Debug, Default)]
pub struct HandlerOutput {
    pub messages: Vec<InternalServerMessage>,
    pub reactions: Vec<Reaction>,
    pub finalize_games: Vec<GameFinalize>,
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
        }
    }
}

/// Bots ask for JSON with `/ws/?format=json` so they never have to reproduce rmp-serde's
/// representation by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketFormat {
    Msgpack,
    Json,
}

#[derive(Clone, Debug)]
pub struct SocketTx {
    pub socket_id: Uuid,
    pub format: SocketFormat,
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

enum Source<'a> {
    Typed(&'a ServerResult),
    Encoded(Bytes),
}

/// Both fields are lazy and live for a single dispatch, so a fanout of browsers encodes
/// msgpack once and never touches `serde_json`. `Encoded` is the fallback for producers that
/// still hand over bytes: those are the only ones that pay a decode, and only when a JSON
/// socket is in the fanout.
pub struct Outbound<'a> {
    source: Source<'a>,
    msgpack: Option<Bytes>,
    json: Option<Bytes>,
}

impl<'a> Outbound<'a> {
    pub fn typed(message: &'a ServerResult) -> Self {
        Self {
            source: Source::Typed(message),
            msgpack: None,
            json: None,
        }
    }

    pub fn encoded(bytes: Bytes) -> Self {
        Self {
            source: Source::Encoded(bytes),
            msgpack: None,
            json: None,
        }
    }

    pub fn bytes(&mut self, format: SocketFormat) -> Option<&Bytes> {
        match format {
            SocketFormat::Msgpack => {
                if self.msgpack.is_none() {
                    self.msgpack = match &self.source {
                        Source::Typed(message) => {
                            MsgpackSerdeCodec::encode(*message).ok().map(Bytes::from)
                        }
                        Source::Encoded(bytes) => Some(bytes.clone()),
                    };
                }
                self.msgpack.as_ref()
            }
            SocketFormat::Json => {
                if self.json.is_none() {
                    self.json = match &self.source {
                        Source::Typed(message) => serde_json::to_vec(message).ok().map(Bytes::from),
                        Source::Encoded(bytes) => MsgpackSerdeCodec::decode(bytes)
                            .ok()
                            .and_then(|message: ServerResult| serde_json::to_vec(&message).ok())
                            .map(Bytes::from),
                    };
                }
                self.json.as_ref()
            }
        }
    }
}

#[cfg(test)]
mod outbound_tests {
    use super::*;

    fn message() -> ServerResult {
        ServerResult::Ok(Box::new(ServerMessage::Error("boom".to_string())))
    }

    #[test]
    fn a_msgpack_socket_gets_exactly_what_it_got_before() {
        let message = message();
        let expected = Bytes::from(MsgpackSerdeCodec::encode(&message).expect("encodes"));

        assert_eq!(
            Outbound::typed(&message).bytes(SocketFormat::Msgpack),
            Some(&expected)
        );
    }

    #[test]
    fn each_format_is_encoded_once_and_reused() {
        let message = message();
        let mut outbound = Outbound::typed(&message);

        let first = outbound.bytes(SocketFormat::Json).cloned();
        let second = outbound.bytes(SocketFormat::Json).cloned();

        assert_eq!(first, second);
        assert_ne!(first, outbound.bytes(SocketFormat::Msgpack).cloned());
    }

    #[test]
    fn an_unmigrated_producers_bytes_pass_through_untouched() {
        let encoded = Bytes::from(MsgpackSerdeCodec::encode(&message()).expect("encodes"));

        assert_eq!(
            Outbound::encoded(encoded.clone()).bytes(SocketFormat::Msgpack),
            Some(&encoded)
        );
    }

    #[test]
    fn an_unmigrated_producers_bytes_still_reach_a_json_socket() {
        let encoded = Bytes::from(MsgpackSerdeCodec::encode(&message()).expect("encodes"));
        let mut outbound = Outbound::encoded(encoded);

        let json = outbound
            .bytes(SocketFormat::Json)
            .expect("transcodes")
            .clone();

        assert_eq!(
            json,
            Bytes::from(serde_json::to_vec(&message()).expect("encodes"))
        );
    }
}

#[derive(Clone, Debug)]
pub struct SocketHandle {
    pub tx: mpsc::Sender<Bytes>,
    pub format: SocketFormat,
}
