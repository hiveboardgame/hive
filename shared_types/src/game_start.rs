use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Hash)]
pub enum GameStart {
    Ready,
    Arena,
    Immediate,
    Moves,
}

impl fmt::Display for GameStart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GameStart::Ready => "Ready",
                GameStart::Arena => "Arena",
                GameStart::Moves => "Moves",
                GameStart::Immediate => "Immediate",
            }
        )
    }
}

use thiserror::Error;
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum GameStartError {
    #[error("{found} is not a valid GameStart")]
    InvalidGameStart { found: String },
}

impl std::str::FromStr for GameStart {
    type Err = GameStartError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ready" => Ok(GameStart::Ready),
            "Arena" => Ok(GameStart::Arena),
            "Immediate" => Ok(GameStart::Immediate),
            "Moves" => Ok(GameStart::Moves),
            s => Err(GameStartError::InvalidGameStart {
                found: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn arena_start_round_trips_without_ready_semantics() {
        assert_eq!(GameStart::Arena.to_string(), "Arena");
        assert_eq!(GameStart::from_str("Arena").unwrap(), GameStart::Arena);
    }
}
