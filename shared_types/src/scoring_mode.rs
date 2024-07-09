use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ScoringMode {
    Game,
    Match,
}

impl fmt::Display for ScoringMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scoring = match self {
            ScoringMode::Game => "Game",
            ScoringMode::Match => "Match",
        };
        write!(f, "{}", scoring)
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ScoringModeError {
    #[error("{found} is not a valid ScoringMode")]
    Invalid { found: String },
}

impl FromStr for ScoringMode {
    type Err = ScoringModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Game" => Ok(ScoringMode::Game),
            "Match" => Ok(ScoringMode::Match),
            s => Err(ScoringModeError::Invalid {
                found: s.to_string(),
            }),
        }
    }
}
