use crate::{
    db_error::DbError,
    models::{Tournament, TournamentSlot},
    tournaments::FixedFieldCommit,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel_async::AsyncConnection;
use shared_types::{tournament::Format, TournamentStatus};
use uuid::Uuid;

use super::{
    super::{
        progress_elimination_game,
        progress_round_robin_game,
        progress_swiss_game,
        reconciliation_slot_ids,
        state::{invalid_input, invalid_persisted, load_in_progress, BotUsers},
        ProgressionEffects,
        TournamentState,
    },
    fixed_field_commit,
};

pub async fn reconcile(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<Option<FixedFieldCommit>, DbError> {
    reconcile_fixed_field_with_time(tournament_id, None, conn).await
}

pub async fn reconcile_locked(
    tournament: Tournament,
    conn: &mut DbConn<'_>,
) -> Result<(Tournament, Option<FixedFieldCommit>), DbError> {
    let mut reconciled = fold_committed_terminal_games_locked(tournament, &[], conn).await?;
    let commit = take_reconciliation_effects(&mut reconciled.state, reconciled.effects);
    Ok((reconciled.state.tournament, commit))
}

#[cfg(test)]
pub(crate) async fn reconcile_fixed_field_at(
    tournament_id: Uuid,
    reconciled_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Option<FixedFieldCommit>, DbError> {
    reconcile_fixed_field_with_time(tournament_id, Some(reconciled_at), conn).await
}

async fn reconcile_fixed_field_with_time(
    tournament_id: Uuid,
    reconciled_at: Option<DateTime<Utc>>,
    conn: &mut DbConn<'_>,
) -> Result<Option<FixedFieldCommit>, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        if tournament.status() == TournamentStatus::Finished {
            return Ok(None);
        }
        let mut reconciled =
            fold_committed_terminal_games_locked_with_time(tournament, &[], reconciled_at, tc)
                .await?;
        Ok(take_reconciliation_effects(
            &mut reconciled.state,
            reconciled.effects,
        ))
    })
    .await
}

pub(super) struct LockedReconciliation {
    pub(super) state: TournamentState,
    pub(super) effects: ProgressionEffects,
}

pub(super) async fn fold_committed_terminal_games_locked(
    tournament: Tournament,
    additional_slot_ids: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<LockedReconciliation, DbError> {
    fold_committed_terminal_games_locked_with_time(tournament, additional_slot_ids, None, conn)
        .await
}

async fn fold_committed_terminal_games_locked_with_time(
    tournament: Tournament,
    additional_slot_ids: &[Uuid],
    reconciled_at: Option<DateTime<Utc>>,
    conn: &mut DbConn<'_>,
) -> Result<LockedReconciliation, DbError> {
    let tournament_id = tournament.id;
    match tournament.status() {
        TournamentStatus::NotStarted => {
            return Err(DbError::InvalidAction {
                info: String::from(
                    "Tournament must be in progress before results can be reconciled",
                ),
            })
        }
        TournamentStatus::Finished => {
            return Err(DbError::InvalidAction {
                info: String::from("Tournament mutation requires an in-progress tournament"),
            })
        }
        TournamentStatus::InProgress => {}
    }
    match tournament.configuration().format() {
        Format::RoundRobin
        | Format::Swiss
        | Format::DoubleSwiss
        | Format::SingleElimination
        | Format::DoubleElimination => {}
        format => {
            return Err(invalid_input(&format!(
                "The {format} tournament adapter does not support fixed-field reconciliation",
            )))
        }
    }

    // Progression can release or auto-resolve existing successors, and finish
    // cleanup clears scheduling fields on resolved rows too. Lock the complete
    // persisted Slot closure before applying any aggregate transition.
    let mut locked_slot_ids = reconciliation_slot_ids(tournament_id, conn).await?;
    locked_slot_ids.extend_from_slice(additional_slot_ids);
    locked_slot_ids.sort_unstable();
    locked_slot_ids.dedup();
    let locked_slots =
        TournamentSlot::find_by_ids_for_update(tournament_id, &locked_slot_ids, conn).await?;
    let reloaded_slot_ids = locked_slots.iter().map(|slot| slot.id).collect::<Vec<_>>();
    if reloaded_slot_ids != locked_slot_ids {
        return Err(DbError::SerializationConflict);
    }
    let reconciled_at = reconciled_at.unwrap_or_else(Utc::now);

    let mut state = load_in_progress(tournament, BotUsers::ForGameRelease, conn).await?;
    let mut pending = state
        .games
        .iter()
        .filter_map(|game| {
            let slot_id = game.tournament_slot_id?;
            (game.finished
                && state.slots[state.slot_index(slot_id).expect("locked Slot was reloaded")]
                    .resolution
                    .is_none())
            .then_some((slot_id, game))
        })
        .map(|(slot_id, game)| {
            let finished_at = game.finished_at.ok_or_else(|| {
                invalid_persisted("a terminal tournament game has no finished_at")
            })?;
            Ok((slot_id, game.id, finished_at))
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    pending.sort_unstable_by_key(|(slot_id, game_id, _)| (*slot_id, *game_id));
    let mut reconciled = ProgressionEffects::default();
    for (slot_id, game_id, finished_at) in pending {
        if state.slots[state.slot_index(slot_id)?].resolution.is_some() {
            continue;
        }
        let mut progressed = progress_terminal_game_in_state_at(
            &mut state,
            game_id,
            finished_at,
            reconciled_at,
            conn,
        )
        .await?;
        reconciled.sealed |= progressed.sealed;
        reconciled.advanced |= progressed.advanced;
        reconciled.finished_now |= progressed.finished_now;
        reconciled
            .released_games
            .append(&mut progressed.released_games);
    }
    if reconciled.finished_now {
        state.tournament = Tournament::find(tournament_id, conn).await?;
    }
    Ok(LockedReconciliation {
        state,
        effects: reconciled,
    })
}

pub(super) fn merge_reconciliation_effects(
    state: &mut TournamentState,
    reconciled: ProgressionEffects,
    outcome: &mut Option<FixedFieldCommit>,
) {
    let Some(reconciled) = take_reconciliation_effects(state, reconciled) else {
        return;
    };
    match outcome {
        Some(current) => current.merge(reconciled),
        None => *outcome = Some(reconciled),
    }
}

fn take_reconciliation_effects(
    state: &mut TournamentState,
    reconciled: ProgressionEffects,
) -> Option<FixedFieldCommit> {
    let progression_changed = reconciled.sealed
        || reconciled.advanced
        || !reconciled.released_games.is_empty()
        || reconciled.finished_now;
    let commit = fixed_field_commit(state, reconciled.finished_now);
    let changed = progression_changed
        || !commit.affected_slot_ids.is_empty()
        || commit.standings_changed
        || commit.format_changed
        || commit.availability_changed
        || commit.catalog_changed
        || !commit.schedule_updates.is_empty();
    changed.then_some(commit)
}

pub(super) async fn progress_terminal_game_in_state(
    state: &mut TournamentState,
    game_id: Uuid,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    progress_terminal_game_in_state_at(state, game_id, progressed_at, progressed_at, conn).await
}

async fn progress_terminal_game_in_state_at(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    match state.configuration.format() {
        Format::RoundRobin => {
            progress_round_robin_game(state, game_id, sealed_at, progressed_at, conn).await
        }
        Format::Swiss | Format::DoubleSwiss => {
            progress_swiss_game(state, game_id, sealed_at, progressed_at, conn).await
        }
        Format::SingleElimination | Format::DoubleElimination => {
            progress_elimination_game(state, game_id, sealed_at, progressed_at, conn).await
        }
        format => Err(invalid_input(&format!(
            "The {format} tournament adapter does not support fixed-field progression",
        ))),
    }
}
