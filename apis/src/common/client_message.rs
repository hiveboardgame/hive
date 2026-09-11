use super::{
    challenge_action::ChallengeAction,
    game_action::GameAction,
    ScheduleAction,
    TournamentAction,
};
use serde::{Deserialize, Serialize};
use shared_types::{ConversationKey, GameId};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSendRequest {
    pub key: ConversationKey,
    pub client_id: Uuid,
    pub body: String,
    pub turn: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionAttempt {
    pub key: ConversationKey,
    pub session_epoch: u64,
    pub request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Chat(ChatSendRequest),
    ChatSubscribe(SubscriptionAttempt),
    ChatUnsubscribe(ConversationKey),
    Challenge(ChallengeAction),
    Game { game_id: GameId, action: GameAction },
    LinkDiscord,
    NotificationSeen { game_id: GameId },
    Pong(u64),
    Resync,
    Schedule(ScheduleAction),
    Tournament(TournamentAction),
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
    // being established/torn down
    // These names are the wire contract: msgpack tags each variant with its name, so renaming
    // one breaks every deployed client. Order is free.
    Auth(String),
    GetGame(GameId),
    GetPendingGames,
    GetUser(Uuid),
    GetUsername(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use codee::{binary::MsgpackSerdeCodec, Encoder};

    /// A bot in any language hand-builds these bytes, so the names are API, not an
    /// implementation detail. Order is not: msgpack tags by name, never by index.
    #[test]
    fn variants_go_on_the_wire_under_their_own_names() {
        let encode = |request: &ClientRequest| MsgpackSerdeCodec::encode(request).expect("encodes");

        assert_eq!(
            encode(&ClientRequest::Auth("tok".to_string())),
            b"\x81\xa4Auth\xa3tok".to_vec()
        );
        assert_eq!(
            encode(&ClientRequest::GetGame(GameId("abc".to_string()))),
            b"\x81\xa7GetGame\xa3abc".to_vec()
        );
        assert_eq!(
            encode(&ClientRequest::GetPendingGames),
            b"\xafGetPendingGames".to_vec()
        );
    }
    #[test]
    fn probe_wire() {
        use crate::common::{ChallengeAction, GameAction};
        let cases: Vec<(&str, ClientRequest)> = vec![
            (
                "Game/Play",
                ClientRequest::Game {
                    game_id: GameId("abc".to_string()),
                    action: GameAction::Play("wA1 -bQ".to_string()),
                },
            ),
            (
                "Game/Join",
                ClientRequest::Game {
                    game_id: GameId("abc".to_string()),
                    action: GameAction::Join,
                },
            ),
            ("GetUser", ClientRequest::GetUser(uuid::Uuid::nil())),
            (
                "Challenge/Accept",
                ClientRequest::Challenge(ChallengeAction::Accept(shared_types::ChallengeId(
                    "xyz".to_string(),
                ))),
            ),
        ];
        for (name, request) in cases {
            let bytes = MsgpackSerdeCodec::encode(&request).expect("encodes");
            println!(
                "{name}: {}",
                bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            println!(
                "{name} ascii: {}",
                bytes
                    .iter()
                    .map(|b| if *b >= 0x20 && *b < 0x7f {
                        (*b as char).to_string()
                    } else {
                        '.'.to_string()
                    })
                    .collect::<String>()
            );
        }
    }

    /// The shapes published in `BOT_WEBSOCKET_API.md`.
    #[test]
    fn json_frames_match_the_documented_shapes() {
        let json = |request: &ClientRequest| serde_json::to_string(request).expect("encodes");

        assert_eq!(
            json(&ClientRequest::Auth("tok".to_string())),
            r#"{"Auth":"tok"}"#
        );
        assert_eq!(
            json(&ClientRequest::GetPendingGames),
            r#""GetPendingGames""#
        );
        assert_eq!(
            json(&ClientRequest::GetGame(GameId("abc".to_string()))),
            r#"{"GetGame":"abc"}"#
        );
        assert_eq!(
            json(&ClientRequest::GetUsername("bot".to_string())),
            r#"{"GetUsername":"bot"}"#
        );
        assert_eq!(
            json(&ClientRequest::Game {
                game_id: GameId("abc".to_string()),
                action: crate::common::GameAction::Play("wA1 -bQ".to_string()),
            }),
            r#"{"Game":{"game_id":"abc","action":{"Play":"wA1 -bQ"}}}"#
        );
    }
}
