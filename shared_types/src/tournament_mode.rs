use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::PrettyString;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum TournamentMode {
    #[default]
    DoubleRoundRobin,
    QuadrupleRoundRobin,
}

impl PrettyString for TournamentMode {
    fn pretty_string(&self) -> String {
        match self {
            Self::DoubleRoundRobin => String::from("Double round robin"),
            Self::QuadrupleRoundRobin => String::from("Quadruple round robin"),
        }
    }
}

impl fmt::Display for TournamentMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let game_status = match self {
            Self::DoubleRoundRobin => String::from("DoubleRoundRobin"),
            Self::QuadrupleRoundRobin => String::from("QuadrupleRoundRobin"),
        };
        write!(f, "{game_status}")
    }
}

impl FromStr for TournamentMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DoubleRoundRobin" => Ok(TournamentMode::DoubleRoundRobin),
            "QuadrupleRoundRobin" => Ok(TournamentMode::QuadrupleRoundRobin),
            _ => Err(anyhow::anyhow!("Invalid TournamentMode string")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_game_status() {
        {
            let ts = TournamentMode::DoubleRoundRobin;
            assert_eq!(
                ts.clone(),
                TournamentMode::from_str(&format!("{ts}")).unwrap()
            );
        }
    }
}
