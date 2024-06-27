use super::game_action::GameAction;
use super::{challenge_action::ChallengeAction, TournamentAction};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_types::{ChatMessageContainer, GameId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    UserSearch(String),
    Chat(ChatMessageContainer),
    Challenge(ChallengeAction),
    Game { game_id: GameId, action: GameAction },
    Ping(DateTime<Utc>),
    Tournament(TournamentAction),
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
          // being established/torn down
          // TODO: all the other things the API does right now
}
