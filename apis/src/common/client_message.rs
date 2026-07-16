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
          // TODO: all the other things the API does right now
}
