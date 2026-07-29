use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

use crate::PrettyString;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TournamentMode {
    SingleRoundRobin,
    #[default]
    DoubleRoundRobin,
    QuadrupleRoundRobin,
    SextupleRoundRobin,
    DutchSwiss,
    BursteinSwiss,
    DoubleSwiss,
    SingleElimination,
    DoubleElimination,
    Arena,
}

impl TournamentMode {
    /// An arena has no rounds at all: players are re-paired the moment their
    /// own game ends, and may join, pause or leave while the clock runs.
    pub fn is_arena(&self) -> bool {
        matches!(self, Self::Arena)
    }

    /// Whether the field is fixed when the tournament starts. Only an arena
    /// admits players afterwards.
    pub fn field_closes_at_start(&self) -> bool {
        !self.is_arena()
    }

    /// How many times every pair meets, for the round-robin modes.
    pub fn round_robin_repeats(&self) -> Option<usize> {
        match self {
            Self::SingleRoundRobin => Some(1),
            Self::DoubleRoundRobin => Some(2),
            Self::QuadrupleRoundRobin => Some(4),
            Self::SextupleRoundRobin => Some(6),
            _ => None,
        }
    }

    pub fn is_round_robin(&self) -> bool {
        self.round_robin_repeats().is_some()
    }

    pub fn is_swiss(&self) -> bool {
        matches!(
            self,
            Self::DutchSwiss | Self::BursteinSwiss | Self::DoubleSwiss
        )
    }

    pub fn is_elimination(&self) -> bool {
        matches!(self, Self::SingleElimination | Self::DoubleElimination)
    }

    /// Whether a pairing is played as two colour-swapped games scored as one
    /// match, rather than a single game.
    pub fn is_two_game_match(&self) -> bool {
        matches!(
            self,
            Self::DoubleSwiss | Self::SingleElimination | Self::DoubleElimination
        )
    }

    /// Whether the organizer's `rounds` is meaningful, or the round count falls
    /// out of the format and the field size instead.
    pub fn rounds_are_chosen(&self) -> bool {
        self.is_swiss()
    }
}

impl PrettyString for TournamentMode {
    fn pretty_string(&self) -> String {
        match self {
            Self::SingleRoundRobin => String::from("Single round robin"),
            Self::DoubleRoundRobin => String::from("Double round robin"),
            Self::QuadrupleRoundRobin => String::from("Quadruple round robin"),
            Self::SextupleRoundRobin => String::from("Sextuple round robin"),
            Self::DutchSwiss => String::from("Swiss (Dutch)"),
            Self::BursteinSwiss => String::from("Swiss (Burstein)"),
            Self::DoubleSwiss => String::from("Double Swiss"),
            Self::SingleElimination => String::from("Single elimination"),
            Self::DoubleElimination => String::from("Double elimination"),
            Self::Arena => String::from("Arena"),
        }
    }
}

impl fmt::Display for TournamentMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mode = match self {
            Self::SingleRoundRobin => "SingleRoundRobin",
            Self::DoubleRoundRobin => "DoubleRoundRobin",
            Self::QuadrupleRoundRobin => "QuadrupleRoundRobin",
            Self::SextupleRoundRobin => "SextupleRoundRobin",
            Self::DutchSwiss => "DutchSwiss",
            Self::BursteinSwiss => "BursteinSwiss",
            Self::DoubleSwiss => "DoubleSwiss",
            Self::SingleElimination => "SingleElimination",
            Self::DoubleElimination => "DoubleElimination",
            Self::Arena => "Arena",
        };
        write!(f, "{mode}")
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum TournamentModeError {
    #[error("{found} is not a valid TournamentMode")]
    Invalid { found: String },
}

impl FromStr for TournamentMode {
    type Err = TournamentModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SingleRoundRobin" => Ok(Self::SingleRoundRobin),
            "DoubleRoundRobin" => Ok(Self::DoubleRoundRobin),
            "QuadrupleRoundRobin" => Ok(Self::QuadrupleRoundRobin),
            "SextupleRoundRobin" => Ok(Self::SextupleRoundRobin),
            "DutchSwiss" => Ok(Self::DutchSwiss),
            "BursteinSwiss" => Ok(Self::BursteinSwiss),
            "DoubleSwiss" => Ok(Self::DoubleSwiss),
            "SingleElimination" => Ok(Self::SingleElimination),
            "DoubleElimination" => Ok(Self::DoubleElimination),
            "Arena" => Ok(Self::Arena),
            s => Err(TournamentModeError::Invalid {
                found: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [TournamentMode; 10] = [
        TournamentMode::SingleRoundRobin,
        TournamentMode::DoubleRoundRobin,
        TournamentMode::QuadrupleRoundRobin,
        TournamentMode::SextupleRoundRobin,
        TournamentMode::DutchSwiss,
        TournamentMode::BursteinSwiss,
        TournamentMode::DoubleSwiss,
        TournamentMode::SingleElimination,
        TournamentMode::DoubleElimination,
        TournamentMode::Arena,
    ];

    #[test]
    fn every_mode_round_trips_through_its_string_encoding() {
        for mode in ALL {
            assert_eq!(mode, TournamentMode::from_str(&format!("{mode}")).unwrap());
        }
    }

    #[test]
    fn every_mode_belongs_to_exactly_one_family() {
        for mode in ALL {
            let families = [
                mode.is_round_robin(),
                mode.is_swiss(),
                mode.is_elimination(),
                mode.is_arena(),
            ];
            assert_eq!(
                families.iter().filter(|belongs| **belongs).count(),
                1,
                "{mode} must belong to exactly one family"
            );
        }
    }

    #[test]
    fn round_robin_repeats_match_the_variant_names() {
        assert_eq!(
            TournamentMode::SingleRoundRobin.round_robin_repeats(),
            Some(1)
        );
        assert_eq!(
            TournamentMode::DoubleRoundRobin.round_robin_repeats(),
            Some(2)
        );
        assert_eq!(
            TournamentMode::QuadrupleRoundRobin.round_robin_repeats(),
            Some(4)
        );
        assert_eq!(
            TournamentMode::SextupleRoundRobin.round_robin_repeats(),
            Some(6)
        );
        assert_eq!(TournamentMode::DoubleSwiss.round_robin_repeats(), None);
    }
}
