use crate::color::Color;
use crate::game_error::GameError;
use crate::game_result::GameResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum GameStatus {
    #[default]
    NotStarted,
    InProgress,
    Finished(GameResult),
}

impl fmt::Display for GameStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let game_status = match self {
            Self::NotStarted => "NotStarted".to_owned(),
            Self::InProgress => "InProgress".to_owned(),
            Self::Finished(result) => format!("Finished({})", result.clone()),
        };
        write!(f, "{game_status}")
    }
}

impl FromStr for GameStatus {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NotStarted" => Ok(GameStatus::NotStarted),
            "InProgress" => Ok(GameStatus::InProgress),
            "Finished(Winner(b))" => Ok(GameStatus::Finished(GameResult::Winner(Color::Black))),
            "Finished(Winner(w))" => Ok(GameStatus::Finished(GameResult::Winner(Color::White))),
            "Finished(Draw)" => Ok(GameStatus::Finished(GameResult::Draw)),
            "Finished(Unknown)" => Ok(GameStatus::Finished(GameResult::Unknown)),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "GameStatus string".to_string(),
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
            GameStatus::NotStarted,
            GameStatus::InProgress,
            GameStatus::Finished(GameResult::Winner(Color::White)),
            GameStatus::Finished(GameResult::Winner(Color::Black)),
            GameStatus::Finished(GameResult::Draw),
            GameStatus::Finished(GameResult::Unknown),
        ]
        .iter()
        {
            assert_eq!(Ok(gc.clone()), GameStatus::from_str(&format!("{gc}")));
        }
    }
}
