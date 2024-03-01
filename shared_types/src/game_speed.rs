use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Hash)]
pub enum GameSpeed {
    Bullet,
    Blitz,
    Rapid,
    Classic,
    Correspondence,
    Untimed,
    Puzzle,
}

impl GameSpeed {
    pub fn all_rated() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![Bullet, Blitz, Rapid, Classic, Correspondence]
    }

    pub fn all() -> Vec<GameSpeed> {
        use GameSpeed::*;
        vec![Bullet, Blitz, Rapid, Classic, Correspondence, Untimed]
    }

    pub fn from_base_increment(base: Option<i32>, increment: Option<i32>) -> GameSpeed {
        let total = base.unwrap_or(0) + 40 * increment.unwrap_or(0);
        if total == 0 {
            GameSpeed::Untimed
        } else if total < 180 {
            GameSpeed::Bullet
        } else if total < 480 {
            GameSpeed::Blitz
        } else if total < 1500 {
            GameSpeed::Rapid
        } else if total < 18000 {
            GameSpeed::Classic
        } else {
            GameSpeed::Correspondence
        }
    }
}

impl fmt::Display for GameSpeed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = match self {
            GameSpeed::Bullet => "Bullet",
            GameSpeed::Blitz => "Blitz",
            GameSpeed::Rapid => "Rapid",
            GameSpeed::Classic => "Classic",
            GameSpeed::Correspondence => "Correspondence",
            GameSpeed::Untimed => "Untimed",
            GameSpeed::Puzzle => "Puzzle",
        };
        write!(f, "{}", time)
    }
}

use thiserror::Error;
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum GameSpeedError {
    #[error("{found} is not a valid GameSpeed")]
    InvalidGameSpeed { found: String },
}

impl std::str::FromStr for GameSpeed {
    type Err = GameSpeedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Bullet" => Ok(GameSpeed::Bullet),
            "Blitz" => Ok(GameSpeed::Blitz),
            "Rapid" => Ok(GameSpeed::Rapid),
            "Classic" => Ok(GameSpeed::Classic),
            "Correspondence" => Ok(GameSpeed::Correspondence),
            "Untimed" => Ok(GameSpeed::Untimed),
            "Puzzle" => Ok(GameSpeed::Untimed),
            s => Err(GameSpeedError::InvalidGameSpeed {
                found: s.to_string(),
            }),
        }
    }
}
