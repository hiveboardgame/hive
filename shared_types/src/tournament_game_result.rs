use anyhow::anyhow;
use hive_lib::{Color, GameResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub enum TournamentGameResult {
    #[default]
    Unknown,
    Draw,
    Winner(Color),
    DoubeForfeit,
    Bye,
}

impl TournamentGameResult {
    pub fn new(game_result: &GameResult) -> Self {
        let mut result = TournamentGameResult::Unknown;
        match game_result {
            GameResult::Winner(color) => result = TournamentGameResult::Winner(*color),
            GameResult::Draw => result = TournamentGameResult::Draw,
            _ => {}
        }
        result
    }
}

impl fmt::Display for TournamentGameResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let game_result = match self {
            Self::Unknown => "Unknown".to_owned(),
            Self::Draw => "½-½".to_owned(),
            Self::Winner(color) => match color {
                Color::Black => "0-1".to_owned(),
                Color::White => "1-0".to_owned(),
            },
            Self::DoubeForfeit => "0-0".to_owned(),
            Self::Bye => "BYE".to_owned(),
        };
        write!(f, "{game_result}")
    }
}

impl FromStr for TournamentGameResult {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unknown" => Ok(TournamentGameResult::Unknown),
            "0-1" => Ok(TournamentGameResult::Winner(Color::Black)),
            "1-0" => Ok(TournamentGameResult::Winner(Color::White)),
            "½-½" => Ok(TournamentGameResult::Draw),
            "0-0" => Ok(TournamentGameResult::DoubeForfeit),
            "BYE" => Ok(TournamentGameResult::Bye),
            _ => Err(anyhow!("Invalid TournamentGameResult string".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_tournament_game_status() {
        for gc in [
            TournamentGameResult::Winner(Color::White),
            TournamentGameResult::Winner(Color::Black),
            TournamentGameResult::Draw,
            TournamentGameResult::Unknown,
        ]
        .iter()
        {
            assert_eq!(
                gc.clone(),
                TournamentGameResult::from_str(&format!("{gc}")).unwrap()
            );
        }
    }
}
