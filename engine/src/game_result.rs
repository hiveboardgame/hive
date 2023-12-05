use crate::color::Color;
use crate::game_error::GameError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GameResult {
    Winner(Color),
    Draw,
    #[default]
    Unknown,
}

impl fmt::Display for GameResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let game_result = match self {
            Self::Unknown => "Unknown".to_owned(),
            Self::Draw => "½-½".to_owned(),
            Self::Winner(color) => match color {
                Color::Black => "0-1".to_owned(),
                Color::White => "1-0".to_owned(),
            },
        };
        write!(f, "{game_result}")
    }
}

impl FromStr for GameResult {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unknown" => Ok(GameResult::Unknown),
            "0-1" => Ok(GameResult::Winner(Color::Black)),
            "1-0" => Ok(GameResult::Winner(Color::White)),
            "½-½" => Ok(GameResult::Draw),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "GameResult string".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_game_status() {
        for gc in [
            GameResult::Winner(Color::White),
            GameResult::Winner(Color::Black),
            GameResult::Draw,
            GameResult::Unknown,
        ]
        .iter()
        {
            assert_eq!(Ok(gc.clone()), GameResult::from_str(&format!("{gc}")));
        }
    }
}
