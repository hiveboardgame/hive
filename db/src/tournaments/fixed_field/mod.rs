//! Transactional workflows for adapted fixed-field tournaments.

mod adjudication;
mod closeout;
mod correction;
mod progression;
pub(crate) mod withdrawal;

pub use adjudication::adjudicate_slot_atomic;
pub use closeout::close_unstarted_slots;
pub use progression::reconcile;
#[cfg(test)]
pub(crate) use progression::reconcile_fixed_field_at;
pub(crate) use progression::reconcile_locked;
pub use withdrawal::withdraw_player;

use crate::{db_error::DbError, DbConn};
use chrono::{DateTime, Utc};
use hive_lib::Color;
use shared_types::{
    tournament::{AdjudicatedGameOutcome, AdjudicatedSideResult, Format, GameOutcome},
    TournamentGameResult,
};
use uuid::Uuid;

use super::{
    evaluate_capabilities,
    finish_fixed_field,
    project_elimination_facts,
    project_round_robin_facts,
    project_swiss_facts,
    seal_elimination_terminal_for_batch,
    seal_round_robin_terminal_for_batch,
    seal_swiss_terminal_for_batch,
    state::{invalid_input, TournamentState},
    CapabilityProjection,
    FormatFacts,
    SlotSealedEvent,
};

fn fixed_field_commit(state: &mut TournamentState, finished_now: bool) -> FixedFieldCommit {
    if finished_now {
        state.record_finish_cleanup();
    }
    let changes = state.take_fixed_field_changes();
    FixedFieldCommit {
        tournament_id: state.tournament.id,
        affected_slot_ids: changes.affected_slot_ids,
        released_game_ids: changes.released_game_ids,
        standings_changed: changes.standings_changed,
        format_changed: changes.format_changed,
        availability_changed: changes.availability_changed,
        finished_now,
        catalog_changed: changes.catalog_changed || finished_now,
        schedule_updates: std::mem::take(&mut state.schedule_offer_updates),
    }
}

fn command_capabilities(state: &TournamentState) -> Result<CapabilityProjection, DbError> {
    match state.configuration.format() {
        Format::RoundRobin => {
            let projected = project_round_robin_facts(state)?;
            evaluate_capabilities(state, FormatFacts::RoundRobin(&projected))
        }
        Format::Swiss | Format::DoubleSwiss => evaluate_capabilities(state, FormatFacts::Swiss),
        Format::SingleElimination | Format::DoubleElimination => {
            let projected = project_elimination_facts(state)?;
            evaluate_capabilities(state, FormatFacts::Elimination(&projected))
        }
        Format::Arena => Ok(CapabilityProjection::default()),
    }
}

fn ensure_adapted_format(state: &TournamentState) -> Result<Format, DbError> {
    let format = state.configuration.format();
    if !supports_format(format) {
        return Err(invalid_input(&format!(
            "The {format} tournament adapter does not support fixed-field commands",
        )));
    }
    Ok(format)
}

async fn seal_terminal_for_batch(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    match ensure_adapted_format(state)? {
        Format::RoundRobin => {
            seal_round_robin_terminal_for_batch(state, game_id, sealed_at, sealed_at, conn).await
        }
        Format::Swiss | Format::DoubleSwiss => {
            seal_swiss_terminal_for_batch(state, game_id, sealed_at, sealed_at, conn).await
        }
        Format::SingleElimination | Format::DoubleElimination => {
            seal_elimination_terminal_for_batch(state, game_id, sealed_at, sealed_at, conn).await
        }
        Format::Arena => Ok(false),
    }
}

async fn finish_automatically_if_required(
    state: &TournamentState,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    let automatic_finish_required = match ensure_adapted_format(state)? {
        Format::RoundRobin => {
            let projection = project_round_robin_facts(state)?;
            projection.is_complete()
        }
        Format::Swiss | Format::DoubleSwiss => {
            let projected = project_swiss_facts(state)?;
            projected.tournament_complete()
        }
        Format::SingleElimination | Format::DoubleElimination => {
            let projected = project_elimination_facts(state)?;
            !projected.has_active_node() && projected.projection.complete
        }
        Format::Arena => false,
    };
    if !automatic_finish_required {
        return Ok(false);
    }
    finish_fixed_field(state, progressed_at, conn).await?;
    Ok(true)
}

fn adjudicated_outcome(result: &TournamentGameResult) -> Result<GameOutcome, DbError> {
    let (white, black) = match result {
        TournamentGameResult::Unknown => {
            return Err(invalid_input(
                "An initial or replacement adjudication requires a concrete result",
            ))
        }
        TournamentGameResult::Draw => (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw),
        TournamentGameResult::Winner(Color::White) => (
            AdjudicatedSideResult::ForfeitWin,
            AdjudicatedSideResult::ForfeitLoss,
        ),
        TournamentGameResult::Winner(Color::Black) => (
            AdjudicatedSideResult::ForfeitLoss,
            AdjudicatedSideResult::ForfeitWin,
        ),
        TournamentGameResult::DoubleForfeit => (
            AdjudicatedSideResult::DoubleForfeit,
            AdjudicatedSideResult::DoubleForfeit,
        ),
    };
    AdjudicatedGameOutcome::new(white, black)
        .map(GameOutcome::Adjudicated)
        .map_err(|error| invalid_input(&format!("Invalid adjudicated outcome: {error}")))
}

pub(crate) const ADAPTED_FORMATS: [Format; 5] = [
    Format::RoundRobin,
    Format::Swiss,
    Format::DoubleSwiss,
    Format::SingleElimination,
    Format::DoubleElimination,
];

pub(crate) const fn supports_format(format: Format) -> bool {
    matches!(
        format,
        Format::RoundRobin
            | Format::Swiss
            | Format::DoubleSwiss
            | Format::SingleElimination
            | Format::DoubleElimination
    )
}

#[derive(Debug)]
pub struct AdjudicationOutcome {
    pub cleared: bool,
    pub newly_terminal: bool,
    pub committed_game: Option<Game>,
    pub commit: Option<FixedFieldCommit>,
}

#[derive(Debug)]
pub struct CloseoutOutcome {
    pub closed_slots: u32,
    pub terminal_games: Vec<Game>,
    pub commit: Option<FixedFieldCommit>,
}

#[derive(Debug)]
pub struct WithdrawalOutcome {
    pub applied: bool,
    pub terminal_games: Vec<Game>,
    pub commit: Option<FixedFieldCommit>,
}

pub const fn supports_closeout(format: Format) -> bool {
    matches!(format, Format::RoundRobin | Format::Swiss)
}

pub async fn scheduled_start_candidates(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    Tournament::due_scheduled_start_ids(&ADAPTED_FORMATS, as_of, limit, conn).await
}

pub async fn timeout_candidates(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    expired_fixed_field_game_ids(&ADAPTED_FORMATS, as_of, limit, conn).await
}

pub async fn reconciliation_candidates(
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    fixed_field_reconciliation_candidate_ids(&ADAPTED_FORMATS, limit, conn).await
}

pub use super::start::{
    cancel_elimination_start,
    confirm_elimination_start,
    expire_elimination_setups,
    prepare_elimination_start,
    prepare_elimination_start_in_transaction,
    start_by_organizer,
    start_scheduled,
    StartOutcome,
};
pub(crate) use super::transition::reconciliation_slot_ids;
use super::{
    transition::{expired_fixed_field_game_ids, fixed_field_reconciliation_candidate_ids},
    FixedFieldCommit,
};
use crate::models::{Game, Tournament};
