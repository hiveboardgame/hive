use crate::TournamentMode;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use thiserror::Error;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
pub enum Tiebreaker {
    /// The tournament's primary score, per its `ScoringMode`: match points for
    /// `Match`, game points for `Game`.
    RawPoints,
    MatchPoints,
    GamePoints,
    Buchholz,
    BuchholzCut1,
    BuchholzCut2,
    BuchholzMedian,
    BuchholzBuchholz,
    Koya,
    SonnebornBerger,
    ProgressiveScore,
    Wins,
    HeadToHead,
    WinsAsBlack,
    /// Elimination only: how far into the bracket a player got.
    RoundsSurvived,
    GamesPlayed,
    Draws,
    Losses,
    /// Arena only: the streak machinery that makes consecutive wins worth
    /// double, and how often a player berserked.
    CurrentStreak,
    BestStreak,
    Berserks,
}

impl Tiebreaker {
    /// The name in full, for anywhere there is room for it — picking
    /// tiebreakers, explaining them. `pretty_str` is the column abbreviation a
    /// standings table needs and is unreadable on its own.
    pub fn full_name(&self) -> &str {
        match self {
            Self::RawPoints => "Points",
            Self::MatchPoints => "Match points",
            Self::GamePoints => "Game points",
            Self::Buchholz => "Buchholz",
            Self::BuchholzCut1 => "Buchholz cut 1",
            Self::BuchholzCut2 => "Buchholz cut 2",
            Self::BuchholzMedian => "Median Buchholz",
            Self::BuchholzBuchholz => "Buchholz of Buchholz",
            Self::Koya => "Koya",
            Self::SonnebornBerger => "Sonneborn-Berger",
            Self::ProgressiveScore => "Progressive score",
            Self::Wins => "Wins",
            Self::HeadToHead => "Direct encounter",
            Self::WinsAsBlack => "Wins as black",
            Self::RoundsSurvived => "Rounds survived",
            Self::Berserks => "Berserks",
            Self::BestStreak => "Best streak",
            Self::CurrentStreak => "Current streak",
            Self::Losses => "Losses",
            Self::Draws => "Draws",
            Self::GamesPlayed => "Games played",
        }
    }

    pub fn pretty_str(&self) -> &str {
        match self {
            Self::RawPoints => "Points",
            Self::MatchPoints => "MP",
            Self::GamePoints => "GP",
            Self::Buchholz => "Bch",
            Self::BuchholzCut1 => "Bch-1",
            Self::BuchholzCut2 => "Bch-2",
            Self::BuchholzMedian => "Bch-Med",
            Self::BuchholzBuchholz => "BchBch",
            Self::Koya => "Koya",
            Self::SonnebornBerger => "SB",
            Self::ProgressiveScore => "Prog",
            Self::Wins => "W",
            Self::HeadToHead => "H2H",
            Self::WinsAsBlack => "WB",
            Self::RoundsSurvived => "Rnd",
            Self::Berserks => "Bsrk",
            Self::BestStreak => "Best",
            Self::CurrentStreak => "Strk",
            Self::Losses => "L",
            Self::Draws => "D",
            Self::GamesPlayed => "Games",
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
        let tiebreaker = match self {
            Self::RawPoints => "RawPoints",
            Self::MatchPoints => "MatchPoints",
            Self::GamePoints => "GamePoints",
            Self::Buchholz => "Buchholz",
            Self::BuchholzCut1 => "BuchholzCut1",
            Self::BuchholzCut2 => "BuchholzCut2",
            Self::BuchholzMedian => "BuchholzMedian",
            Self::BuchholzBuchholz => "BuchholzBuchholz",
            Self::Koya => "Koya",
            Self::SonnebornBerger => "SonnebornBerger",
            Self::ProgressiveScore => "ProgressiveScore",
            Self::Wins => "Wins",
            Self::HeadToHead => "HeadToHead",
            Self::WinsAsBlack => "WinsAsBlack",
            Self::RoundsSurvived => "RoundsSurvived",
            Self::Berserks => "Berserks",
            Self::BestStreak => "BestStreak",
            Self::CurrentStreak => "CurrentStreak",
            Self::Losses => "Losses",
            Self::Draws => "Draws",
            Self::GamesPlayed => "GamesPlayed",
        };
        write!(f, "{tiebreaker}")
    }
}

impl Tiebreaker {
    /// The tiebreakers an organizer may choose for a mode, in the order they
    /// are offered.
    ///
    /// Empty for arenas and brackets, and that is not an oversight: an arena
    /// ranks on points, then wins, then fewest games — the engine's own order,
    /// already applied to the groups it returns — and a bracket has no score at
    /// all beyond how far you got. Neither has anything left to break a tie
    /// with, so offering a choice would be offering a lie.
    ///
    /// `RawPoints` is absent throughout because it is always the primary and is
    /// prepended by `Tournament::standings`; it is never a *tie* breaker.
    pub fn available_for(mode: TournamentMode) -> Vec<Self> {
        if mode.is_arena() || mode.is_elimination() {
            return Vec::new();
        }
        let mut available = vec![
            Self::HeadToHead,
            Self::SonnebornBerger,
            Self::Buchholz,
            Self::BuchholzCut1,
            Self::BuchholzCut2,
            Self::BuchholzMedian,
            Self::BuchholzBuchholz,
            Self::Koya,
            Self::ProgressiveScore,
            Self::Wins,
            Self::WinsAsBlack,
        ];
        // Only a two-game match can separate these: a 2-0 and a 1½-½ are the
        // same match result but different game scores.
        if mode.is_two_game_match() {
            available.insert(0, Self::GamePoints);
            available.insert(1, Self::MatchPoints);
        }
        available
    }

    /// What a mode starts with, before the organizer changes anything.
    ///
    /// Round robin leaves Buchholz out on purpose: everyone plays everyone, so
    /// every player's opponents are very nearly the same set and the sum says
    /// almost nothing. Direct encounter and Sonneborn-Berger are what actually
    /// separate a round robin.
    pub fn defaults_for(mode: TournamentMode) -> Vec<Self> {
        if mode.is_arena() || mode.is_elimination() {
            return Vec::new();
        }
        if mode.is_swiss() {
            let mut defaults = vec![
                Self::Buchholz,
                Self::BuchholzCut1,
                Self::SonnebornBerger,
                Self::ProgressiveScore,
            ];
            if mode.is_two_game_match() {
                defaults.insert(0, Self::GamePoints);
            }
            return defaults;
        }
        vec![Self::HeadToHead, Self::SonnebornBerger, Self::WinsAsBlack]
    }
}

impl FromStr for Tiebreaker {
    type Err = TiebreakerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RawPoints" => Ok(Self::RawPoints),
            "MatchPoints" => Ok(Self::MatchPoints),
            "GamePoints" => Ok(Self::GamePoints),
            "Buchholz" => Ok(Self::Buchholz),
            "BuchholzCut1" => Ok(Self::BuchholzCut1),
            "BuchholzCut2" => Ok(Self::BuchholzCut2),
            "BuchholzMedian" => Ok(Self::BuchholzMedian),
            "BuchholzBuchholz" => Ok(Self::BuchholzBuchholz),
            "Koya" => Ok(Self::Koya),
            "SonnebornBerger" => Ok(Self::SonnebornBerger),
            "ProgressiveScore" => Ok(Self::ProgressiveScore),
            "Wins" => Ok(Self::Wins),
            "HeadToHead" => Ok(Self::HeadToHead),
            "WinsAsBlack" => Ok(Self::WinsAsBlack),
            "RoundsSurvived" => Ok(Self::RoundsSurvived),
            "Berserks" => Ok(Self::Berserks),
            "BestStreak" => Ok(Self::BestStreak),
            "CurrentStreak" => Ok(Self::CurrentStreak),
            "Losses" => Ok(Self::Losses),
            "Draws" => Ok(Self::Draws),
            "GamesPlayed" => Ok(Self::GamesPlayed),
            s => Err(TiebreakerError::InvalidTiebreaker {
                found: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tiebreaker_round_trips_through_its_string_encoding() {
        for tiebreaker in [
            Tiebreaker::RawPoints,
            Tiebreaker::MatchPoints,
            Tiebreaker::GamePoints,
            Tiebreaker::Buchholz,
            Tiebreaker::BuchholzCut1,
            Tiebreaker::BuchholzCut2,
            Tiebreaker::BuchholzMedian,
            Tiebreaker::BuchholzBuchholz,
            Tiebreaker::Koya,
            Tiebreaker::SonnebornBerger,
            Tiebreaker::ProgressiveScore,
            Tiebreaker::Wins,
            Tiebreaker::HeadToHead,
            Tiebreaker::WinsAsBlack,
            Tiebreaker::RoundsSurvived,
            Tiebreaker::Berserks,
            Tiebreaker::BestStreak,
            Tiebreaker::CurrentStreak,
            Tiebreaker::Losses,
            Tiebreaker::Draws,
            Tiebreaker::GamesPlayed,
        ] {
            assert_eq!(
                tiebreaker,
                Tiebreaker::from_str(&format!("{tiebreaker}")).unwrap()
            );
        }
    }
}
