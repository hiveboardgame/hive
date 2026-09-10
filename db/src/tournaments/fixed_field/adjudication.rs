use crate::{
    db_error::DbError,
    models::{Game, Tournament, TournamentSlot, User},
    tournaments::fixed_field::AdjudicationOutcome,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel_async::AsyncConnection;
use shared_types::{
    tournament::{Format, Resolution},
    Conclusion,
    SlotAdminAction,
    TournamentGameResult,
};
use uuid::Uuid;

use super::{
    super::{
        ensure_planned_double_swiss_second_adjudication,
        progress_swiss,
        round_robin::{
            release_sequential_successor_after_disposition,
            sequential_slot_is_release_eligible,
        },
        seal_slot,
        state::{ProgressionEffects, TournamentState},
    },
    adjudicated_outcome,
    command_capabilities,
    correction::correct_fixed_field_slot,
    ensure_adapted_format,
    finish_automatically_if_required,
    fixed_field_commit,
    progression::{
        fold_committed_terminal_games_locked,
        merge_reconciliation_effects,
        progress_terminal_game_in_state,
    },
    SlotSealedEvent,
};

/// Administratively settles or corrects one adapted fixed-field slot.
///
/// Initial adjudication and semantic progression commit in one transaction, as
/// do corrections.
pub async fn adjudicate_slot_atomic(
    tournament_id: Uuid,
    slot_id: Uuid,
    result: TournamentGameResult,
    expected_resolution: Option<Resolution>,
    actor_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<AdjudicationOutcome, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        User::ensure_active_ids(&[actor_id], tc).await?;
        tournament
            .ensure_user_is_organizer_or_admin(&actor_id, tc)
            .await?;
        let additional_slot_ids = if matches!(
            tournament.configuration().format(),
            Format::SingleElimination | Format::DoubleElimination
        ) {
            TournamentSlot::find_by_tournament_id(tournament_id, tc)
                .await?
                .into_iter()
                .map(|slot| slot.id)
                .collect::<Vec<_>>()
        } else {
            vec![slot_id]
        };
        let reconciled =
            fold_committed_terminal_games_locked(tournament, &additional_slot_ids, tc).await?;
        let mut state = reconciled.state;
        let reconciliation_effects = reconciled.effects;
        ensure_adapted_format(&state)?;
        let locked_slot = TournamentSlot::find_for_update(tournament_id, slot_id, tc).await?;
        let slot_index = state.slot_index(slot_id)?;
        state.slots[slot_index] = locked_slot;
        let linked_game_ids = state
            .slot_game(slot_id)
            .map(|game| vec![game.id])
            .unwrap_or_default();
        for locked_game in Game::find_by_ids_for_update(&linked_game_ids, tc).await? {
            if let Some(game) = state
                .games
                .iter_mut()
                .find(|game| game.id == locked_game.id)
            {
                *game = locked_game;
            }
        }
        let current_resolution = state.slots[state.slot_index(slot_id)?].resolution;
        let desired_resolution = if result == TournamentGameResult::Unknown {
            None
        } else {
            Some(Resolution::Result(adjudicated_outcome(&result)?))
        };
        if current_resolution != expected_resolution {
            if current_resolution == desired_resolution {
                let mut outcome = AdjudicationOutcome {
                    cleared: false,
                    newly_terminal: false,
                    committed_game: None,
                    commit: None,
                };
                merge_reconciliation_effects(
                    &mut state,
                    reconciliation_effects,
                    &mut outcome.commit,
                );
                return Ok(outcome);
            }
            return Err(DbError::InvalidAction {
                info: String::from("Tournament slot result changed before adjudication"),
            });
        }
        let effective_at = Utc::now();
        let mut outcome = match state.slots[state.slot_index(slot_id)?].resolution {
            Some(Resolution::Result(_)) => {
                correct_fixed_field_slot(&mut state, slot_id, result, effective_at, tc).await
            }
            None => {
                adjudicate_initial_fixed_field_slot(&mut state, slot_id, result, effective_at, tc)
                    .await
            }
            Some(Resolution::Withdrawal(_) | Resolution::Clinched) => Err(DbError::InvalidAction {
                info: String::from(
                    "Played, withdrawal, and clinched results cannot be adjudicated",
                ),
            }),
        }?;
        merge_reconciliation_effects(&mut state, reconciliation_effects, &mut outcome.commit);
        Ok(outcome)
    })
    .await
}

async fn adjudicate_initial_fixed_field_slot(
    state: &mut TournamentState,
    slot_id: Uuid,
    result: TournamentGameResult,
    adjudicated_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<AdjudicationOutcome, DbError> {
    let requested_result = result.clone();
    let format = ensure_adapted_format(state)?;
    let slot_index = state.slot_index(slot_id)?;
    if requested_result == TournamentGameResult::Unknown {
        if state.slots[slot_index].resolution.is_some() {
            return Err(DbError::InvalidAction {
                info: String::from("A terminal fixed-field slot requires a result correction"),
            });
        }
        return Ok(AdjudicationOutcome {
            cleared: false,
            newly_terminal: false,
            committed_game: None,
            commit: None,
        });
    }
    let capabilities = command_capabilities(state)?;
    if !capabilities
        .slots
        .get(&slot_id)
        .is_some_and(|slot| slot.admin_actions.contains(&SlotAdminAction::RecordResult))
    {
        return Err(DbError::InvalidAction {
            info: String::from("This tournament result can no longer be recorded"),
        });
    }
    let outcome = adjudicated_outcome(&requested_result)?;
    let game_id = match state.slot_game(slot_id).map(|game| game.id) {
        Some(game_id) if state.slots[slot_index].resolution.is_none() => game_id,
        None if state.slots[slot_index].resolution.is_none() => {
            let release_round_robin_successor = match format {
                Format::RoundRobin => sequential_slot_is_release_eligible(state, slot_id)?,
                Format::DoubleSwiss => {
                    ensure_planned_double_swiss_second_adjudication(state, slot_id)?;
                    false
                }
                Format::Swiss => {
                    return Err(DbError::InvalidAction {
                        info: String::from("Swiss adjudication requires its current game"),
                    })
                }
                Format::SingleElimination | Format::DoubleElimination => {
                    return Err(DbError::InvalidAction {
                        info: String::from(
                            "Elimination adjudication cannot bypass the current series game",
                        ),
                    })
                }
                Format::Arena => false,
            };
            seal_slot(
                state,
                SlotSealedEvent {
                    slot_id,
                    outcome,
                    sealed_at: adjudicated_at,
                },
                conn,
            )
            .await?;
            let progressed = match format {
                Format::RoundRobin => {
                    let released_games = if release_round_robin_successor {
                        release_sequential_successor_after_disposition(
                            state,
                            slot_id,
                            adjudicated_at,
                            conn,
                        )
                        .await?
                    } else {
                        Vec::new()
                    };
                    let finished_now =
                        finish_automatically_if_required(state, adjudicated_at, conn).await?;
                    ProgressionEffects {
                        released_games,
                        finished_now,
                        ..ProgressionEffects::default()
                    }
                }
                Format::DoubleSwiss => progress_swiss(state, adjudicated_at, conn).await?,
                _ => ProgressionEffects::default(),
            };
            return Ok(AdjudicationOutcome {
                cleared: false,
                newly_terminal: true,
                committed_game: None,
                commit: Some(fixed_field_commit(state, progressed.finished_now)),
            });
        }
        _ => {
            return Err(DbError::InvalidAction {
                info: String::from(
                    "A terminal fixed-field slot requires an exact result correction",
                ),
            })
        }
    };
    let game = state
        .game(game_id)?
        .adjudicate_unstarted(
            &requested_result,
            Conclusion::Committee,
            adjudicated_at,
            conn,
        )
        .await?;
    if let Some(current) = state
        .games
        .iter_mut()
        .find(|candidate| candidate.id == game.id)
    {
        *current = game.clone();
    }
    let progressed = progress_terminal_game_in_state(state, game.id, adjudicated_at, conn).await?;
    Ok(AdjudicationOutcome {
        cleared: false,
        newly_terminal: true,
        committed_game: Some(game),
        commit: Some(fixed_field_commit(state, progressed.finished_now)),
    })
}
