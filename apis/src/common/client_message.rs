use super::{
    challenge_action::ChallengeAction,
    game_action::GameAction,
    ScheduleAction,
    TournamentAction,
};
use serde::{Deserialize, Serialize};
use shared_types::{ChannelKey, ChatMessageContainer, GameId};
use urlencoding::encode;
use uuid::Uuid;

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
    pub fn error_field_for_user(&self, current_user_id: Uuid) -> String {
        match self {
            ClientRequest::Chat(container) => {
                let key = ChannelKey::from_destination_for_user(
                    &container.destination,
                    current_user_id,
                );
                format!(
                    "chat:{}:{}",
                    key.channel_type.as_str(),
                    encode(&key.channel_id),
                )
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
