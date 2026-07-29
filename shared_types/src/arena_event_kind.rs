use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

/// What a player did to their own participation in a running arena. Joining is
/// not here: it is recorded by `tournaments_users.joined_at`, which every
/// entrant has exactly one of.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum ArenaEventKind {
    /// Step out of the pairing pool. Requested mid-game, it takes effect when
    /// that game ends — the game still counts.
    Pause,
    /// Step back in, or call off a break that has not started yet.
    Resume,
    /// Leave for good. Results already earned still stand and still count
    /// toward everyone else's standings.
    Withdraw,
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ArenaEventKindError {
    #[error("{found} is not a valid ArenaEventKind")]
    Invalid { found: String },
}

impl fmt::Display for ArenaEventKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match self {
            Self::Pause => "Pause",
            Self::Resume => "Resume",
            Self::Withdraw => "Withdraw",
        };
        write!(f, "{kind}")
    }
}

impl FromStr for ArenaEventKind {
    type Err = ArenaEventKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pause" => Ok(Self::Pause),
            "Resume" => Ok(Self::Resume),
            "Withdraw" => Ok(Self::Withdraw),
            s => Err(ArenaEventKindError::Invalid {
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
        for kind in [
            ArenaEventKind::Pause,
            ArenaEventKind::Resume,
            ArenaEventKind::Withdraw,
        ] {
            assert_eq!(kind, ArenaEventKind::from_str(&format!("{kind}")).unwrap());
        }
    }
}
