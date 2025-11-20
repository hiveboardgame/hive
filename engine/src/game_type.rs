use crate::game_error::GameError;
use serde::Serialize;
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

impl GameType {
    pub(crate) fn add_m(self) -> Self {
        match self {
            GameType::Base => GameType::M,
            GameType::L => GameType::ML,
            GameType::P => GameType::MP,
            GameType::LP => GameType::MLP,
            _ => self,
        }
    }

    pub(crate) fn add_l(self) -> Self {
        match self {
            GameType::Base => GameType::L,
            GameType::M => GameType::ML,
            GameType::P => GameType::LP,
            GameType::MP => GameType::MLP,
            _ => self,
        }
    }

    pub(crate) fn add_p(self) -> Self {
        match self {
            GameType::Base => GameType::P,
            GameType::M => GameType::MP,
            GameType::L => GameType::LP,
            GameType::ML => GameType::MLP,
            _ => self,
        }
    }
}
