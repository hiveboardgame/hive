use shared_types::challenge_error::ChallengeError;
use hive_lib::{color::ColorChoice, game_type::GameType};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeAction {
    Accept(String), // The user accepts the challenge identified by the nanoid
    Create {
        rated: bool,
        game_type: GameType,
        visibility: ChallengeVisibility,
        opponent: Option<String>,
        color_choice: ColorChoice,
        time_mode: String,
        time_base: Option<i32>,
        time_increment: Option<i32>,
    },
    Decline(String), // Deletes the direct challenge with nanoid
    Delete(String),  // Deletes the challenge with nanoid
    Get(String),     // Gets one challenge
    GetOwn,          // All of the user's open challenges (public, private, direct)
    GetDirected,     // Challenges directed at you
    GetPublic,       // Get public challenges (minus own)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeVisibility {
    Direct,
    Public,
    Private,
}

impl fmt::Display for ChallengeVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Private => write!(f, "Private"),
            Self::Public => write!(f, "Public"),
            Self::Direct => write!(f, "Direct"),
        }
    }
}

impl FromStr for ChallengeVisibility {
    type Err = ChallengeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Private" => Ok(ChallengeVisibility::Private),
            "Public" => Ok(ChallengeVisibility::Public),
            "Direct" => Ok(ChallengeVisibility::Direct),
            s => Err(ChallengeError::InvalidChallengeVisibility {
                found: s.to_string(),
            }),
        }
    }
}
