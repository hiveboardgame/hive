use crate::{db_error::DbError, tournaments::fixed_field::AdjudicationOutcome, DbConn};
use chrono::{DateTime, Utc};
use shared_types::{
    tournament::{Format, GameOutcome, Resolution},
    SlotAdminAction,
    TournamentGameResult,
};
use uuid::Uuid;

use super::{
    super::{
        correct_slot,
        progress_elimination_for_node,
        release_cleared_double_swiss_second_if_eligible,
        reopen_elimination_after_result_correction,
        round_robin::release_cleared_sequential_slot_if_eligible,
        state::TournamentState,
    },
    adjudicated_outcome,
    command_capabilities,
    ensure_adapted_format,
    finish_automatically_if_required,
    fixed_field_commit,
};

pub(super) async fn correct_fixed_field_slot(
    state: &mut TournamentState,
    slot_id: Uuid,
    result: TournamentGameResult,
    corrected_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<AdjudicationOutcome, DbError> {
    let (cleared, replacement) = match &result {
        TournamentGameResult::Unknown => (true, None),
        concrete => (false, Some(adjudicated_outcome(concrete)?)),
    };

    let format = ensure_adapted_format(state)?;
    let slot_index = state.slot_index(slot_id)?;
    let (current_outcome, prior_game_id) = match state.slots[slot_index].resolution {
        Some(Resolution::Result(outcome)) => {
            (outcome, state.slot_game(slot_id).map(|game| game.id))
        }
        None | Some(Resolution::Withdrawal(_) | Resolution::Clinched) => {
            return Err(DbError::InvalidAction {
                info: String::from("A fixed-field result correction requires a sealed slot"),
            })
        }
    };
    if !matches!(current_outcome, GameOutcome::Adjudicated(_)) {
        return Err(DbError::InvalidAction {
            info: String::from("A played fixed-field result cannot be corrected"),
        });
    }
    if replacement == Some(current_outcome) {
        return Ok(AdjudicationOutcome {
            cleared: false,
            newly_terminal: false,
            committed_game: None,
            commit: None,
        });
    }
    let required_action = if cleared {
        SlotAdminAction::ClearResult
    } else {
        SlotAdminAction::ReplaceResult
    };
    let capabilities = command_capabilities(state)?;
    if !capabilities
        .slots
        .get(&slot_id)
        .is_some_and(|slot| slot.admin_actions.contains(&required_action))
    {
        return Err(DbError::InvalidAction {
            info: String::from("This tournament result can no longer be corrected"),
        });
    }
    correct_slot(
        state,
        slot_id,
        replacement,
        (!cleared).then_some(result),
        corrected_at,
        conn,
    )
    .await?;
    let corrected_elimination_node = if matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    ) {
        Some(reopen_elimination_after_result_correction(state, slot_id, corrected_at, conn).await?)
    } else {
        None
    };

    match format {
        Format::RoundRobin if cleared && prior_game_id.is_none() => {
            release_cleared_sequential_slot_if_eligible(state, slot_id, corrected_at, conn).await?;
        }
        Format::DoubleSwiss if cleared && prior_game_id.is_none() => {
            release_cleared_double_swiss_second_if_eligible(state, slot_id, corrected_at, conn)
                .await?;
        }
        _ => {}
    }
    let game = match prior_game_id {
        Some(game_id) => Some(state.game(game_id)?.clone()),
        None => None,
    };

    let finished_now = if matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    ) {
        progress_elimination_for_node(state, corrected_elimination_node, corrected_at, conn)
            .await?
            .finished_now
    } else {
        finish_automatically_if_required(state, corrected_at, conn).await?
    };
    Ok(AdjudicationOutcome {
        cleared,
        newly_terminal: false,
        committed_game: game,
        commit: Some(fixed_field_commit(state, finished_now)),
    })
}
