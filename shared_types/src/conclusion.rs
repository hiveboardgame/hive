use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Conclusion {
    Board,
    Committee,
    Draw,
    Forfeit,
    Repetition,
    Resigned,
    Timeout,
    Unknown,
}

impl PrettyString for Conclusion {
    fn pretty_string(&self) -> String {
        match self {
            Conclusion::Board => String::from("Finished on board"),
            Conclusion::Committee => String::from("Committee decision"),
            Conclusion::Draw => String::from("Draw agreed"),
            Conclusion::Forfeit => String::from("Forfeit"),
            Conclusion::Repetition => String::from("3 move repetition"),
            Conclusion::Resigned => String::from("Resigned"),
            Conclusion::Timeout => String::from("Timeout"),
            Conclusion::Unknown => String::from("Unknown"),
        }
    }
}

impl fmt::Display for Conclusion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = match self {
            Conclusion::Board => "Board",
            Conclusion::Committee => "Committee",
            Conclusion::Draw => "Draw",
            Conclusion::Repetition => "Repetition",
            Conclusion::Forfeit => "Forfeit",
            Conclusion::Resigned => "Resigned",
            Conclusion::Timeout => "Timeout",
            Conclusion::Unknown => "Unknown",
        };
        write!(f, "{}", time)
    }
}

use thiserror::Error;

use crate::PrettyString;
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ConclusionError {
    #[error("{found} is not a valid Conclusion")]
    InvalidConclusion { found: String },
}

impl std::str::FromStr for Conclusion {
    type Err = ConclusionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Board" => Ok(Conclusion::Board),
            "Committee" => Ok(Conclusion::Committee),
            "Draw" => Ok(Conclusion::Draw),
            "Forfeit" => Ok(Conclusion::Forfeit),
            "Repetition" => Ok(Conclusion::Repetition),
            "Resigned" => Ok(Conclusion::Resigned),
            "Timeout" => Ok(Conclusion::Timeout),
            "Unknown" => Ok(Conclusion::Unknown),
            s => Err(ConclusionError::InvalidConclusion {
                found: s.to_string(),
            }),
        }
    }
}
