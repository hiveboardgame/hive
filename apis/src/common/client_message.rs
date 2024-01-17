use super::challenge_action::ChallengeAction;
use super::game_action::GameAction;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Challenge(ChallengeAction),
    Game { id: String, action: GameAction },
    GameTimeout(String),
    Ping(DateTime<Utc>),
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
          // being established/torn down
          // TODO: all the other things the API does right now
}
