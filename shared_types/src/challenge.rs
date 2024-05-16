use std::{fmt, str::FromStr};

use crate::TimeMode;
use hive_lib::{ColorChoice, GameType};
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeVisibility {
    Direct,
    Public,
    Private,
}
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeError {
    #[error("{found} is not a valid ChallengeVisibility")]
    InvalidChallengeVisibility { found: String },
    #[error("Couldn't find challenge creator (uid {0})")]
    MissingChallenger(String),
    #[error("You can't accept your own challenges!")]
    OwnChallenge,
    #[error("This is not your challenge")]
    NotUserChallenge,
    #[error("{found} is not a valid TimeMode")]
    NotValidTimeMode { found: String },
    #[error("Your rating {rating} is outside the rating band {band_lower}-{band_upper}")]
    OutsideBand {
        rating: u64,
        band_upper: u64,
        band_lower: u64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChallengeDetails {
    pub rated: bool,
    pub game_type: GameType,
    pub visibility: ChallengeVisibility,
    pub opponent: Option<String>,
    pub color_choice: ColorChoice,
    pub time_mode: TimeMode,
    pub time_base: Option<i32>,
    pub time_increment: Option<i32>,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
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
