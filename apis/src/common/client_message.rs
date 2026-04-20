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
    Challenge(ChallengeAction),
    Game { game_id: GameId, action: GameAction },
    LinkDiscord,
    Pong(u64),
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
            ClientRequest::Chat(container) => {
                ConversationKey::from_destination(&container.destination).error_field()
            }
            ClientRequest::Challenge(_) => "challenge".to_string(),
            ClientRequest::Game { .. } => "game".to_string(),
            ClientRequest::LinkDiscord => "link_discord".to_string(),
            ClientRequest::Pong(_) => "pong".to_string(),
            ClientRequest::Schedule(_) => "schedule".to_string(),
            ClientRequest::Tournament(_) => "tournament".to_string(),
            ClientRequest::Away => "presence".to_string(),
        }
    }
}
