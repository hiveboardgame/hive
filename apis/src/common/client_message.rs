use super::{
    challenge_action::ChallengeAction,
    game_action::GameAction,
    ScheduleAction,
    TournamentAction,
};
use serde::{Deserialize, Serialize};
use shared_types::{ConversationKey, GameId, TournamentId};
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
pub enum TvInterest {
    None,
    Lobby,
    Game(GameId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Chat(ChatSendRequest),
    ChatSubscribe(SubscriptionAttempt),
    ChatUnsubscribe(ConversationKey),
    Challenge(ChallengeAction),
    Game { game_id: GameId, action: GameAction },
    NotificationSeen { game_id: GameId },
    Pong(u64),
    Resync,
    Schedule(ScheduleAction),
    Tournament(TournamentAction),
    TournamentWatch(TournamentId),
    TournamentUnwatch(TournamentId),
    TvSubscribe(TvInterest),
    TvUnsubscribe(TvInterest),
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
    // being established/torn down
    // These names are the wire contract: msgpack tags each variant with its name, so renaming
    // one breaks every deployed client. Order is free.
    Auth(String),
    GetGame(GameId),
    GetPendingGames,
    GetOngoingGames,
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
        assert_eq!(
            json(&ClientRequest::Game {
                game_id: GameId("abc".to_string()),
                action: crate::common::GameAction::PlayAt {
                    turn: 12,
                    notation: "wA1 -bQ".to_string(),
                },
            }),
            r#"{"Game":{"game_id":"abc","action":{"PlayAt":{"turn":12,"notation":"wA1 -bQ"}}}}"#
        );
    }

    /// The busiest frame a bot sends, and the one the Python client hard-codes.
    #[test]
    fn a_create_challenge_frame_matches_the_documented_shape() {
        use crate::common::ChallengeAction;
        use hive_lib::{ColorChoice, GameType};
        use shared_types::{ChallengeDetails, ChallengeVisibility, TimeMode};

        let request = ClientRequest::Challenge(ChallengeAction::Create(ChallengeDetails {
            rated: true,
            game_type: GameType::MLP,
            visibility: ChallengeVisibility::Public,
            opponent: None,
            color_choice: ColorChoice::Random,
            time_mode: TimeMode::RealTime,
            time_base: Some(300),
            time_increment: Some(3),
            band_upper: None,
            band_lower: None,
        }));

        assert_eq!(
            serde_json::to_string(&request).expect("encodes"),
            r#"{"Challenge":{"Create":{"rated":true,"game_type":"MLP","visibility":"Public","opponent":null,"color_choice":"Random","time_mode":"RealTime","time_base":300,"time_increment":3,"band_upper":null,"band_lower":null}}}"#
        );
    }
    /// The frames `BOT_WEBSOCKET_API.md` documents beyond the HTTP API, and the replies and
    /// errors a bot parses.
    #[test]
    fn the_wider_bot_surface_matches_the_documented_shapes() {
        use crate::common::{
            ExternalServerError,
            GameUpdate,
            ServerMessage,
            ServerResult,
            TournamentAction,
        };
        use shared_types::GameThread;

        let json = |request: &ClientRequest| serde_json::to_string(request).expect("encodes");
        let game_id = GameId("abc".to_string());

        assert_eq!(
            json(&ClientRequest::GetOngoingGames),
            r#""GetOngoingGames""#
        );
        assert_eq!(json(&ClientRequest::Away), r#""Away""#);
        assert_eq!(
            json(&ClientRequest::Chat(ChatSendRequest {
                key: ConversationKey::Game {
                    game_id: game_id.clone(),
                    thread: GameThread::Players,
                },
                client_id: Uuid::nil(),
                body: "gg".to_string(),
                turn: None,
            })),
            r#"{"Chat":{"key":{"Game":{"game_id":"abc","thread":"players"}},"client_id":"00000000-0000-0000-0000-000000000000","body":"gg","turn":null}}"#
        );
        assert_eq!(
            json(&ClientRequest::ChatSubscribe(SubscriptionAttempt {
                key: ConversationKey::Global,
                session_epoch: 0,
                request_id: 0,
            })),
            r#"{"ChatSubscribe":{"key":"Global","session_epoch":0,"request_id":0}}"#
        );
        assert_eq!(
            json(&ClientRequest::Tournament(TournamentAction::Join(
                TournamentId("t1".to_string())
            ))),
            r#"{"Tournament":{"Join":"t1"}}"#
        );
        assert_eq!(
            json(&ClientRequest::TournamentWatch(TournamentId(
                "t1".to_string()
            ))),
            r#"{"TournamentWatch":"t1"}"#
        );
        assert_eq!(
            json(&ClientRequest::NotificationSeen {
                game_id: game_id.clone()
            }),
            r#"{"NotificationSeen":{"game_id":"abc"}}"#
        );
        assert_eq!(
            json(&ClientRequest::Game {
                game_id: game_id.clone(),
                action: crate::common::GameAction::Berserk,
            }),
            r#"{"Game":{"game_id":"abc","action":"Berserk"}}"#
        );
        assert_eq!(json(&ClientRequest::Pong(123)), r#"{"Pong":123}"#);

        let result = |result: &ServerResult| serde_json::to_string(result).expect("encodes");
        assert_eq!(
            result(&ServerResult::Err(ExternalServerError::Game {
                game_id,
                reason: "Game is over".to_string(),
            })),
            r#"{"Err":{"Game":{"game_id":"abc","reason":"Game is over"}}}"#
        );
        assert_eq!(
            result(&ServerResult::Ok(Box::new(ServerMessage::Game(Box::new(
                GameUpdate::Ongoing(Vec::new())
            ))))),
            r#"{"Ok":{"Game":{"Ongoing":[]}}}"#
        );
        assert_eq!(
            result(&ServerResult::Ok(Box::new(ServerMessage::Error(
                "Already authenticated".to_string()
            )))),
            r#"{"Ok":{"Error":"Already authenticated"}}"#
        );
    }
}
