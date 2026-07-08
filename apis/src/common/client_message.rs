use super::{
    challenge_action::ChallengeAction,
    game_action::GameAction,
    ScheduleAction,
    TournamentAction,
};
use serde::{Deserialize, Serialize};
use shared_types::{ChatMessageContainer, ConversationKey, GameId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Chat(ChatMessageContainer),
    ChatSubscribe(ConversationKey),
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

impl ClientRequest {
    pub fn error_field(&self) -> String {
        match self {
            Self::Chat(container) => {
                ConversationKey::from_destination(&container.destination).error_field()
            }
            Self::ChatSubscribe(_) | Self::ChatUnsubscribe(_) => "chat_subscription".to_string(),
            Self::Challenge(_) => "challenge".to_string(),
            Self::Game { .. } => "game".to_string(),
            Self::LinkDiscord => "link_discord".to_string(),
            Self::NotificationSeen { .. } => "notification".to_string(),
            Self::Pong(_) => "pong".to_string(),
            Self::Resync => "resync".to_string(),
            Self::Schedule(_) => "schedule".to_string(),
            Self::Tournament(_) => "tournament".to_string(),
            Self::Away => "presence".to_string(),
        }
    }

    pub fn chat_client_id(&self) -> Option<uuid::Uuid> {
        match self {
            Self::Chat(container) => container.client_id,
            _ => None,
        }
    }
}
