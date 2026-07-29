use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

/// Why a player sat a round out, which is what decides the bye's score.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Default)]
pub enum ByeKind {
    /// The field had an odd number of pairable players and the engine handed
    /// somebody the bye. Worth a full point by convention.
    #[default]
    PairingAllocated,
    /// The player sat the round out by request. Worth whatever the tournament
    /// set `points_zero_point_bye` to, and it does not count as having taken
    /// part in the round.
    ZeroPoint,
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ByeKindError {
    #[error("{found} is not a valid ByeKind")]
    Invalid { found: String },
}

impl fmt::Display for ByeKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match self {
            Self::PairingAllocated => "PairingAllocated",
            Self::ZeroPoint => "ZeroPoint",
        };
        write!(f, "{kind}")
    }
}

impl FromStr for ByeKind {
    type Err = ByeKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PairingAllocated" => Ok(Self::PairingAllocated),
            "ZeroPoint" => Ok(Self::ZeroPoint),
            s => Err(ByeKindError::Invalid {
                found: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips_through_its_string_encoding() {
        for kind in [ByeKind::PairingAllocated, ByeKind::ZeroPoint] {
            assert_eq!(kind, ByeKind::from_str(&format!("{kind}")).unwrap());
        }
    }
}
