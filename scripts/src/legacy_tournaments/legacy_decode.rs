use super::{
    legacy_standings::LegacyTiebreaker,
    model::{LegacyGameRow, LegacyTournamentRow},
};
use shared_types::{Clock, TimeMode};
use std::str::FromStr;

pub enum ClockDecodeError {
    TimeMode(String),
    Clock(String),
    Untimed,
}

pub fn clock(tournament: &LegacyTournamentRow) -> Result<Clock, ClockDecodeError> {
    let mode = TimeMode::from_str(&tournament.time_mode)
        .map_err(|error| ClockDecodeError::TimeMode(error.to_string()))?;
    Clock::from_time_parts(mode, tournament.time_base, tournament.time_increment)
        .map_err(|error| ClockDecodeError::Clock(error.to_string()))?
        .ok_or(ClockDecodeError::Untimed)
}

pub fn untouched_ready_adjudication(game: &LegacyGameRow) -> bool {
    game.turn == 0
        && game.history.is_empty()
        && game.game_control_history.is_empty()
        && game.hashes.is_empty()
        && game.move_times.is_empty()
        && game.game_start == "Ready"
}

pub enum TiebreakerDecodeError {
    Null,
    Unknown(String),
}

pub fn tiebreaker(stored: Option<&str>) -> Result<LegacyTiebreaker, TiebreakerDecodeError> {
    LegacyTiebreaker::from_str(stored.ok_or(TiebreakerDecodeError::Null)?)
        .map_err(TiebreakerDecodeError::Unknown)
}
