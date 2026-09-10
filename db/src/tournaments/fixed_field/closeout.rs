use crate::{
    db_error::DbError,
    models::{Game, Tournament, TournamentSlot, User},
    tournaments::fixed_field::CloseoutOutcome,
    DbConn,
};
use chrono::Utc;
use diesel_async::AsyncConnection;
use shared_types::{tournament::Format, Conclusion, TournamentGameResult};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{
    super::{progress_swiss, seal_slot, state::ProgressionEffects},
    adjudicated_outcome,
    command_capabilities,
    finish_automatically_if_required,
    fixed_field_commit,
    progression::{fold_committed_terminal_games_locked, merge_reconciliation_effects},
    seal_terminal_for_batch,
    SlotSealedEvent,
};

pub async fn close_unstarted_slots(
    tournament_id: Uuid,
    actor_id: Uuid,
    mut slot_ids: Vec<Uuid>,
    conn: &mut DbConn<'_>,
) -> Result<CloseoutOutcome, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        User::ensure_active_ids(&[actor_id], tc).await?;
        tournament
            .ensure_user_is_organizer_or_admin(&actor_id, tc)
            .await?;
        if slot_ids.is_empty() {
            return Err(DbError::InvalidAction {
                info: String::from("No tournament slots were selected"),
            });
        }
        let requested_count = slot_ids.len();
        slot_ids.sort_unstable();
        if slot_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(DbError::InvalidAction {
                info: String::from("Selected tournament slots contain duplicates"),
            });
        }

        let mutable_slot_ids = TournamentSlot::find_by_tournament_id(tournament_id, tc)
            .await?
            .into_iter()
            .filter(|slot| slot.resolution.is_none())
            .map(|slot| slot.id)
            .collect::<Vec<_>>();
        let reconciled =
            fold_committed_terminal_games_locked(tournament, &mutable_slot_ids, tc).await?;
        let preview = reconciled.state;
        let reconciliation_effects = reconciled.effects;
        let locked_slots =
            TournamentSlot::find_by_ids_for_update(tournament_id, &mutable_slot_ids, tc).await?;
        let mutable_slot_ids = mutable_slot_ids.into_iter().collect::<HashSet<_>>();
        let mutable_game_ids = preview
            .games
            .iter()
            .filter(|game| {
                game.tournament_slot_id
                    .is_some_and(|slot_id| mutable_slot_ids.contains(&slot_id))
            })
            .map(|game| game.id)
            .collect::<Vec<_>>();
        let locked_games = Game::find_by_ids_for_update(&mutable_game_ids, tc).await?;
        let mut state = preview;
        let mut slots_by_id = state
            .slots
            .iter_mut()
            .map(|slot| (slot.id, slot))
            .collect::<HashMap<_, _>>();
        for locked_slot in locked_slots {
            if let Some(slot) = slots_by_id.get_mut(&locked_slot.id) {
                **slot = locked_slot;
            }
        }
        let mut games_by_id = state
            .games
            .iter_mut()
            .map(|game| (game.id, game))
            .collect::<HashMap<_, _>>();
        for locked_game in locked_games {
            if let Some(game) = games_by_id.get_mut(&locked_game.id) {
                **game = locked_game;
            }
        }
        let closed_at = Utc::now();
        let capabilities = command_capabilities(&state)?;
        let candidate_ids = capabilities
            .closeout_slot_ids
            .ok_or_else(|| DbError::InvalidAction {
                info: String::from("Bulk closeout is not valid for this tournament"),
            })?
            .into_iter()
            .collect::<HashSet<_>>();
        if slot_ids
            .iter()
            .any(|slot_id| !candidate_ids.contains(slot_id))
        {
            return Err(DbError::InvalidAction {
                info: String::from("Selected tournament slots are no longer eligible"),
            });
        }

        let format = state.configuration.format();
        let double_forfeit = adjudicated_outcome(&TournamentGameResult::DoubleForfeit)?;
        let mut terminal_games = Vec::new();
        // Batch sealing only replaces games; progression runs after this loop.
        let game_indices_by_slot = state
            .games
            .iter()
            .enumerate()
            .filter_map(|(index, game)| game.tournament_slot_id.map(|slot_id| (slot_id, index)))
            .collect::<HashMap<_, _>>();
        for slot_id in &slot_ids {
            if let Some(&game_index) = game_indices_by_slot.get(slot_id) {
                let game = state.games[game_index]
                    .adjudicate_unstarted(
                        &TournamentGameResult::DoubleForfeit,
                        Conclusion::Forfeit,
                        closed_at,
                        tc,
                    )
                    .await?;
                let game_id = game.id;
                state.games[game_index] = game.clone();
                seal_terminal_for_batch(&mut state, game_id, closed_at, tc).await?;
                terminal_games.push(game);
            } else {
                seal_slot(
                    &mut state,
                    SlotSealedEvent {
                        slot_id: *slot_id,
                        outcome: double_forfeit,
                        sealed_at: closed_at,
                    },
                    tc,
                )
                .await?;
            }
        }

        let progressed = match format {
            Format::RoundRobin => ProgressionEffects {
                finished_now: finish_automatically_if_required(&state, closed_at, tc).await?,
                ..ProgressionEffects::default()
            },
            Format::Swiss => progress_swiss(&mut state, closed_at, tc).await?,
            _ => ProgressionEffects::default(),
        };
        if format == Format::RoundRobin {
            let current_capabilities = command_capabilities(&state)?;
            let changed_slots = capabilities
                .slots
                .iter()
                .filter_map(|(slot_id, before)| {
                    (current_capabilities.slots.get(slot_id) != Some(before)).then_some(*slot_id)
                })
                .collect::<Vec<_>>();
            state.record_affected_slots(changed_slots);
        }
        let mut outcome = CloseoutOutcome {
            closed_slots: u32::try_from(requested_count)
                .expect("creation-bounded Slot count fits u32"),
            terminal_games,
            commit: Some(fixed_field_commit(&mut state, progressed.finished_now)),
        };
        merge_reconciliation_effects(&mut state, reconciliation_effects, &mut outcome.commit);
        Ok(outcome)
    })
    .await
}
