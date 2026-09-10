pub mod arena;
pub mod configuration;
pub mod fixed_field;
pub mod projection;
pub mod public;
mod rating;

use crate::models::ScheduleOffer;
use chrono::{DateTime, Utc};
use shared_types::GameId;
use tournamint::{GameOutcome, Pairing, Score};
use uuid::Uuid;

pub(crate) use configuration::game_time_parts;
pub use configuration::{
    build_double_elimination_bracket,
    build_round_robin_config,
    build_round_robin_definition,
    build_single_elimination_bracket,
    build_swiss_config,
    build_swiss_tournament,
    validate_creation_configuration,
    DefinitionError,
};

#[derive(Debug)]
pub struct FixedFieldCommit {
    pub tournament_id: Uuid,
    pub affected_slot_ids: Vec<Uuid>,
    pub released_game_ids: Vec<GameId>,
    pub standings_changed: bool,
    pub format_changed: bool,
    pub availability_changed: bool,
    pub finished_now: bool,
    pub catalog_changed: bool,
    pub schedule_updates: Vec<ScheduleOffer>,
}

impl FixedFieldCommit {
    pub(crate) fn merge(&mut self, mut other: Self) {
        debug_assert_eq!(self.tournament_id, other.tournament_id);
        self.affected_slot_ids.append(&mut other.affected_slot_ids);
        self.affected_slot_ids.sort_unstable();
        self.affected_slot_ids.dedup();
        self.released_game_ids.append(&mut other.released_game_ids);
        self.released_game_ids
            .sort_unstable_by(|left, right| left.0.cmp(&right.0));
        self.released_game_ids.dedup();
        self.standings_changed |= other.standings_changed;
        self.format_changed |= other.format_changed;
        self.availability_changed |= other.availability_changed;
        self.finished_now |= other.finished_now;
        self.catalog_changed |= other.catalog_changed;
        self.schedule_updates.append(&mut other.schedule_updates);
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlotSealedEvent {
    pub slot_id: Uuid,
    pub outcome: GameOutcome,
    pub sealed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArenaResultSnapshot {
    pub ordinal: i64,
    pub pairing: Pairing,
    pub outcome: GameOutcome,
    pub points: [Score; 2],
    pub doubled: [bool; 2],
    pub ratings: [Option<u32>; 2],
}

mod capabilities;
mod elimination;
mod finished;
mod round_robin;
mod start;
mod state;
mod swiss;
mod transition;
mod withdrawal;

#[cfg(test)]
mod tests;

pub(crate) use capabilities::{
    evaluate_capabilities,
    CapabilityProjection,
    FormatFacts,
    SlotCapabilities,
};

pub(crate) use arena::{arena_projection_for_snapshot, settle_arena_game, ArenaFactsProjection};

pub(crate) use finished::finish_fixed_field;

pub(crate) use elimination::{
    elimination_config,
    elimination_finished_snapshot,
    elimination_player_counts,
    progress_elimination,
    progress_elimination_for_node,
    progress_elimination_game,
    project_elimination_facts,
    public_resolution as public_elimination_resolution,
    public_source as public_elimination_source,
    record_elimination_withdrawal,
    reopen_elimination_after_result_correction,
    seal_elimination_terminal_for_batch,
    series_facts_by_node as elimination_series_facts_by_node,
    EliminationFactsProjection,
};

pub(crate) use round_robin::{
    progress_round_robin_game,
    project_round_robin_facts,
    round_robin_finished_snapshot,
    seal_round_robin_terminal_for_batch,
    sequential_release_eligible_slots,
    RoundRobinFactsProjection,
};
pub(crate) use start::InitialArtifacts;
#[cfg(test)]
pub(crate) use state::load_in_progress_for_update;
pub(crate) use state::{load_for_read_with_memberships, ProgressionEffects, TournamentState};
pub(crate) use swiss::{
    ensure_planned_double_swiss_second_adjudication,
    progress_swiss,
    progress_swiss_game,
    project_swiss_facts,
    release_cleared_double_swiss_second_if_eligible,
    seal_swiss_terminal_for_batch,
    swiss_finished_snapshot,
    swiss_progress,
    SwissFactsProjection,
};

pub(crate) use transition::{
    apply_slot_resolutions,
    apply_withdrawal,
    correct_slot,
    materialize_slots,
    persist_finished_with_outcome,
    reconciliation_slot_ids,
    release_slot,
    seal_slot,
    seal_slot_with_cleanup_at,
    terminal_game_outcome,
    SlotResolutionUpdate,
};
pub(crate) use withdrawal::{withdrawal_obligations, WithdrawalObligation};
