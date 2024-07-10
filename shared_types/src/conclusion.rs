use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Conclusion {
    Unknown,
    Resigned,
    Timeout,
    Draw,
    Board,
    Repetition,
}

impl PrettyString for Conclusion {
    fn pretty_string(&self) -> String {
        match self {
            Conclusion::Board => String::from("Finished on board"),
            Conclusion::Draw => String::from("Draw agreed"),
            Conclusion::Resigned => String::from("Resigned"),
            Conclusion::Timeout => String::from("Timeout"),
            Conclusion::Repetition => String::from("3 move repetition"),
            Conclusion::Unknown => String::from("Unknown"),
        }
    }
}

impl fmt::Display for Conclusion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = match self {
            Conclusion::Unknown => "Unknown",
            Conclusion::Resigned => "Resigned",
            Conclusion::Timeout => "Timeout",
            Conclusion::Draw => "Draw",
            Conclusion::Board => "Board",
            Conclusion::Repetition => "Repetition",
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
            "Unknown" => Ok(Conclusion::Unknown),
            "Resigned" => Ok(Conclusion::Resigned),
            "Timeout" => Ok(Conclusion::Timeout),
            "Draw" => Ok(Conclusion::Draw),
            "Board" => Ok(Conclusion::Board),
            "Repetition" => Ok(Conclusion::Repetition),
            s => Err(ConclusionError::InvalidConclusion {
                found: s.to_string(),
            }),
        }
    }
}
