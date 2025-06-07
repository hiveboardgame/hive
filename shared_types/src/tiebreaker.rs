use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
pub enum Tiebreaker {
    RawPoints,
    HeadToHead,
    WinsAsBlack,
    SonnebornBerger,
    Buchholz,
    BuchholzCut1,
    DirectEncounter,
}

impl Tiebreaker {
    pub fn pretty_str(&self) -> &str {
        match self {
            Tiebreaker::WinsAsBlack => "WB",
            Tiebreaker::HeadToHead => "H2H",
            Tiebreaker::RawPoints => "Points",
            Tiebreaker::SonnebornBerger => "SB",
            Tiebreaker::Buchholz => "Buch",
            Tiebreaker::BuchholzCut1 => "Buch1",
            Tiebreaker::DirectEncounter => "Direct",
        }
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum TiebreakerError {
    #[error("{found} is not a valid Tiebreaker")]
    InvalidTiebreaker { found: String },
}

impl Display for Tiebreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tiebreaker::WinsAsBlack => write!(f, "WinsAsBlack"),
            Tiebreaker::HeadToHead => write!(f, "HeadToHead"),
            Tiebreaker::RawPoints => write!(f, "RawPoints"),
            Tiebreaker::SonnebornBerger => write!(f, "SonnebornBerger"),
            Tiebreaker::Buchholz => write!(f, "Buchholz"),
            Tiebreaker::BuchholzCut1 => write!(f, "BuchholzCut1"),
            Tiebreaker::DirectEncounter => write!(f, "DirectEncounter"),
        }
    }
}

impl FromStr for Tiebreaker {
    type Err = TiebreakerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HeadToHead" => Ok(Tiebreaker::HeadToHead),
            "RawPoints" => Ok(Tiebreaker::RawPoints),
            "WinsAsBlack" => Ok(Tiebreaker::WinsAsBlack),
            "SonnebornBerger" => Ok(Tiebreaker::SonnebornBerger),
            "Buchholz" => Ok(Tiebreaker::Buchholz),
            "BuchholzCut1" => Ok(Tiebreaker::BuchholzCut1),
            "DirectEncounter" => Ok(Tiebreaker::DirectEncounter),
            s => Err(TiebreakerError::InvalidTiebreaker {
                found: s.to_string(),
            }),
        }
    }
}
