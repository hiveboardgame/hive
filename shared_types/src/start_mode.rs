use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum StartMode {
    Full,
    Manual,
    Date,
}

impl fmt::Display for StartMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let start = match self {
            StartMode::Date => "Date",
            StartMode::Manual => "Manual",
            StartMode::Full => "Full",
        };
        write!(f, "{}", start)
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum StartModeError {
    #[error("{found} is not a valid StartMode")]
    Invalid { found: String },
}

impl FromStr for StartMode {
    type Err = StartModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Date" => Ok(StartMode::Date),
            "Manual" => Ok(StartMode::Manual),
            "Full" => Ok(StartMode::Full),
            s => Err(StartModeError::Invalid {
                found: s.to_string(),
            }),
        }
    }
}
