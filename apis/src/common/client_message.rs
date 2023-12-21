use super::game_action::GameAction;
use hive_lib::{color::ColorChoice, game_type::GameType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Challenge(ChallengeAction),
    Game { id: String, action: GameAction },
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
          // being established/torn down
          // TODO: all the other things the API does right now
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeAction {
    Accept(String), // The user accepts the challenge identified by the nanoid
    Delete(String), // Deletes the challenge with nanoid
    Create {
        rated: bool,
        game_type: GameType,
        visibility: ChallengeVisibility,
        color_choice: ColorChoice,
    },
    GetOwn,      // All of the user's open challenges (public, private, direct)
    GetDirected, // CHallenges directed at you
    GetPublic,   // Get public challenges (minus own)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeVisibility {
    Public,
    Private,
    Direct(Uuid),
}
