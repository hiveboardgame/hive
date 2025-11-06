use crate::game_error::GameError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Copy, Default, serde_with::DeserializeFromStr)]
pub enum GameType {
    #[default]
    Base,
    M,
    L,
    P,
    ML,
    LP,
    MP,
    MLP,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub enum GameTypeFilter {
    Base,
    Full,
    #[default]
    All,
}

impl GameTypeFilter {
    pub fn all() -> Vec<Self> {
        vec![Self::Full, Self::Base, Self::All]
    }

    pub fn to_db_filter(&self) -> Vec<&'static str> {
        match self {
            Self::Full => vec!["Base+MLP"],
            Self::Base => vec!["Base"],
            Self::All => vec!["Base+MLP", "Base"],
        }
    }

    pub fn to_sql_filter(&self) -> &'static str {
        match self {
            Self::Full => "AND game_type = 'Base+MLP'",
            Self::Base => "AND game_type = 'Base'",
            Self::All => "AND game_type IN ('Base+MLP', 'Base')",
        }
    }

    pub fn to_string_filter(&self) -> String {
        match self {
            Self::Full => "Full".to_string(),
            Self::Base => "Base".to_string(),
            Self::All => "Full & Base".to_string(),
        }
    }
}

impl fmt::Display for GameTypeFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Full => "Full",
            Self::Base => "Base",
            Self::All => "Full & Base",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for GameTypeFilter {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Full" => Ok(Self::Full),
            "Base" => Ok(Self::Base),
            "Full & Base" => Ok(Self::All),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "game type filter string".to_string(),
            }),
        }
    }
}

impl fmt::Display for GameType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let game_type = match self {
            GameType::Base => "Base",
            GameType::M => "Base+M",
            GameType::L => "Base+L",
            GameType::P => "Base+P",
            GameType::ML => "Base+ML",
            GameType::MP => "Base+MP",
            GameType::LP => "Base+LP",
            GameType::MLP => "Base+MLP",
        };
        write!(f, "{game_type}")
    }
}

impl FromStr for GameType {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Base" => Ok(GameType::Base),
            "Base+M" | "M" => Ok(GameType::M),
            "Base+L" | "L" => Ok(GameType::L),
            "Base+P" | "P" => Ok(GameType::P),
            "Base+ML" | "ML" => Ok(GameType::ML),
            "Base+MP" | "MP" => Ok(GameType::MP),
            "Base+LP" | "LP" => Ok(GameType::LP),
            "Base+MLP" | "MLP" => Ok(GameType::MLP),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "game type string".to_string(),
            }),
        }
    }
}
