use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum MoveConfirm {
    #[default]
    Double,
    Single,
    Clock,
}

impl fmt::Display for MoveConfirm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            MoveConfirm::Clock => "Clock",
            MoveConfirm::Double => "Double",
            MoveConfirm::Single => "Single",
        };
        write!(f, "{}", name)
    }
}

impl FromStr for MoveConfirm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Clock" => Ok(MoveConfirm::Clock),
            "Double" => Ok(MoveConfirm::Double),
            "Single" => Ok(MoveConfirm::Single),
            _ => Err(()),
        }
    }
}
