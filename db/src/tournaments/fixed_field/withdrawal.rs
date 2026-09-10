use crate::{
    db_error::DbError,
    models::{DeadlineSettlement, Game, Rating, Tournament, TournamentSlot, TournamentUser, User},
    tournaments::fixed_field::WithdrawalOutcome,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel_async::AsyncConnection;
use hive_lib::{Color, GameControl, GameResult};
use shared_types::{
    tournament::{AdjudicatedGameOutcome, AdjudicatedSideResult, Format, GameOutcome, Resolution},
    Conclusion,
    TournamentGameResult,
};
use std::str::FromStr;
use tournamint::PlayerId;
use uuid::Uuid;

use super::{
    super::{
        apply_withdrawal,
        progress_elimination,
        progress_swiss,
        record_elimination_withdrawal,
        state::{invalid_input, invalid_persisted, load_in_progress, BotUsers},
        terminal_game_outcome,
        withdrawal_obligations,
        ProgressionEffects,
        SlotResolutionUpdate,
        TournamentState,
        WithdrawalObligation,
    },
    command_capabilities,
    ensure_adapted_format,
    finish_automatically_if_required,
    fixed_field_commit,
    progression::{fold_committed_terminal_games_locked, merge_reconciliation_effects},
    seal_terminal_for_batch,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WithdrawalOrigin {
    UserCommand,
    AccountDeletion,
}

pub(crate) struct Withdrawal {
    state: TournamentState,
    player_id: Uuid,
    slot_ids: Vec<Uuid>,
    game_ids: Vec<Uuid>,
}

impl Withdrawal {
    pub(crate) fn tournament_id(&self) -> Uuid {
        self.state.tournament.id
    }

    pub(crate) fn slot_ids(&self) -> &[Uuid] {
        &self.slot_ids
    }

    pub(crate) fn game_ids(&self) -> &[Uuid] {
        &self.game_ids
    }
}

async fn lock_withdrawal_state(
    mut state: TournamentState,
    player_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<TournamentState, DbError> {
    let tournament_id = state.tournament.id;
    let (mutable_slot_ids, mutable_game_ids) = withdrawal_lock_ids(&state, player_id);
    let locked_slots =
        TournamentSlot::find_by_ids_for_update(tournament_id, &mutable_slot_ids, conn).await?;
    for slot in locked_slots {
        if let Some(index) = state.slots.iter().position(|current| current.id == slot.id) {
            state.slots[index] = slot;
        }
    }
    let locked_games = Game::find_by_ids_for_update(&mutable_game_ids, conn).await?;
    for game in locked_games {
        if let Some(current) = state.games.iter_mut().find(|current| current.id == game.id) {
            *current = game;
        }
    }
    if let Some(membership) =
        TournamentUser::find_for_update(tournament_id, player_id, conn).await?
    {
        if let Some(current) = state
            .memberships
            .iter_mut()
            .find(|current| current.user_id == player_id)
        {
            *current = membership;
        }
    }
    Ok(state)
}

fn withdrawal_lock_ids(state: &TournamentState, player_id: Uuid) -> (Vec<Uuid>, Vec<Uuid>) {
    let slot_ids = withdrawal_slot_lock_ids(state.configuration.format(), &state.slots, player_id);
    let game_ids = state
        .games
        .iter()
        .filter(|game| {
            game.tournament_slot_id
                .is_some_and(|slot_id| slot_ids.contains(&slot_id))
        })
        .map(|game| game.id)
        .collect();
    (slot_ids, game_ids)
}

fn withdrawal_slot_lock_ids(
    format: Format,
    slots: &[TournamentSlot],
    player_id: Uuid,
) -> Vec<Uuid> {
    let elimination = matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    );
    slots
        .iter()
        .filter(|slot| {
            slot.resolution.is_none()
                && (elimination || slot.white == player_id || slot.black == player_id)
        })
        .map(|slot| slot.id)
        .collect()
}

pub(crate) async fn prepare(
    tournament: Tournament,
    player_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<Withdrawal, DbError> {
    let state = load_in_progress(tournament, BotUsers::ForGameRelease, conn).await?;
    ensure_adapted_format(&state)?;
    let (slot_ids, game_ids) = withdrawal_lock_ids(&state, player_id);
    Ok(Withdrawal {
        state,
        player_id,
        slot_ids,
        game_ids,
    })
}

pub(crate) async fn apply(
    mut prepared: Withdrawal,
    locked_slots: &[TournamentSlot],
    locked_games: &[Game],
    withdrawn_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<WithdrawalOutcome, DbError> {
    for slot in locked_slots.iter().filter(|slot| {
        slot.tournament_id == prepared.state.tournament.id && prepared.slot_ids.contains(&slot.id)
    }) {
        if let Some(index) = prepared
            .state
            .slots
            .iter()
            .position(|current| current.id == slot.id)
        {
            prepared.state.slots[index] = slot.clone();
        }
    }
    for game in locked_games
        .iter()
        .filter(|game| prepared.game_ids.contains(&game.id))
    {
        if let Some(current) = prepared
            .state
            .games
            .iter_mut()
            .find(|current| current.id == game.id)
        {
            *current = game.clone();
        }
    }
    if let Some(membership) =
        TournamentUser::find_for_update(prepared.state.tournament.id, prepared.player_id, conn)
            .await?
    {
        if let Some(current) = prepared
            .state
            .memberships
            .iter_mut()
            .find(|current| current.user_id == prepared.player_id)
        {
            *current = membership;
        }
    }
    apply_withdrawal_command(
        &mut prepared.state,
        prepared.player_id,
        withdrawn_at,
        WithdrawalOrigin::AccountDeletion,
        conn,
    )
    .await
}

/// Permanently withdraws one frozen adapted fixed-field entrant.
///
/// Ordinary terminal games and a timeout that wins the withdrawal race are
/// sealed first without releasing lane successors. Every still-open
/// obligation is then settled by one fixed-field withdrawal event, so the
/// command cannot materialize a game that it immediately forfeits.
pub async fn withdraw_player(
    tournament_id: Uuid,
    player_id: Uuid,
    actor_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<WithdrawalOutcome, DbError> {
    conn.transaction::<_, DbError, _>(async move |tc| {
        let tournament = Tournament::find_for_update(tournament_id, tc).await?;
        User::ensure_active_ids(&[actor_id], tc).await?;
        if actor_id != player_id {
            tournament
                .ensure_user_is_organizer_or_admin(&actor_id, tc)
                .await?;
        }
        let withdrawal_slot_ids = withdrawal_slot_lock_ids(
            tournament.configuration().format(),
            &TournamentSlot::find_by_tournament_id(tournament_id, tc).await?,
            player_id,
        );
        let reconciled =
            fold_committed_terminal_games_locked(tournament, &withdrawal_slot_ids, tc).await?;
        let mut state = reconciled.state;
        let reconciliation_effects = reconciled.effects;
        if reconciliation_effects.finished_now {
            let mut commit = None;
            merge_reconciliation_effects(&mut state, reconciliation_effects, &mut commit);
            return Ok(WithdrawalOutcome {
                applied: false,
                terminal_games: Vec::new(),
                commit,
            });
        }
        let mut state = lock_withdrawal_state(state, player_id, tc).await?;
        ensure_adapted_format(&state)?;
        let requested_at = Utc::now();
        let withdrawn_at = withdrawal_instant(&state, requested_at);
        let mut outcome = apply_withdrawal_command(
            &mut state,
            player_id,
            withdrawn_at,
            WithdrawalOrigin::UserCommand,
            tc,
        )
        .await?;
        merge_reconciliation_effects(&mut state, reconciliation_effects, &mut outcome.commit);
        Ok(outcome)
    })
    .await
}

async fn apply_withdrawal_command(
    state: &mut TournamentState,
    player_id: Uuid,
    withdrawn_at: DateTime<Utc>,
    origin: WithdrawalOrigin,
    conn: &mut DbConn<'_>,
) -> Result<WithdrawalOutcome, DbError> {
    let format = ensure_adapted_format(state)?;
    let player = state
        .memberships
        .iter()
        .position(|membership| membership.user_id == player_id)
        .map(PlayerId::new)
        .ok_or_else(|| DbError::InvalidAction {
            info: String::from("The withdrawing user is not in the frozen field"),
        })?;

    if state
        .memberships
        .iter()
        .any(|membership| membership.user_id == player_id && membership.withdrawn_at.is_some())
    {
        let finished_now = finish_automatically_if_required(state, withdrawn_at, conn).await?;
        return Ok(WithdrawalOutcome {
            applied: false,
            terminal_games: Vec::new(),
            commit: finished_now.then(|| fixed_field_commit(state, true)),
        });
    }

    if origin == WithdrawalOrigin::UserCommand
        && !command_capabilities(state)?
            .withdrawable_entrants
            .contains(&player_id)
    {
        return Err(DbError::InvalidAction {
            info: String::from("This entrant can no longer withdraw from the tournament"),
        });
    }

    let mut terminal_games = Vec::new();
    let mut tournament_changed = false;
    'normalize: loop {
        let mut actions = withdrawal_obligations(state, player_id);
        actions.sort_by_key(withdrawal_action_phase);
        let played_resignation_games = actions
            .iter()
            .filter_map(|action| match action {
                WithdrawalObligation::Played { game_id, .. } => Some(*game_id),
                _ => None,
            })
            .map(|game_id| state.game(game_id).cloned())
            .collect::<Result<Vec<_>, DbError>>()?;
        if origin == WithdrawalOrigin::UserCommand {
            Rating::lock_for_game_updates(&played_resignation_games, conn).await?;
        }
        let mut restart_after_terminal_race = false;
        for action in actions {
            match action {
                WithdrawalObligation::Terminal { game_id, .. } => {
                    let game = state.game(game_id)?.clone();
                    let conclusion = parsed_conclusion(&game)?;
                    if conclusion != Conclusion::Withdrawal {
                        tournament_changed |=
                            seal_terminal_for_batch(state, game_id, withdrawn_at, conn).await?;
                        if matches!(
                            format,
                            Format::SingleElimination | Format::DoubleElimination
                        ) {
                            let progressed =
                                progress_elimination(state, withdrawn_at, conn).await?;
                            tournament_changed |= progressed.sealed
                                || progressed.advanced
                                || !progressed.released_games.is_empty()
                                || progressed.finished_now;
                            if progressed.finished_now {
                                return Ok(WithdrawalOutcome {
                                    applied: false,
                                    terminal_games,
                                    commit: Some(fixed_field_commit(state, true)),
                                });
                            }
                            restart_after_terminal_race = true;
                            break;
                        }
                    }
                }
                WithdrawalObligation::Played { game_id, color, .. } => {
                    let game = state.game(game_id)?.clone();
                    let checked = match game.settle_deadline(withdrawn_at, conn).await? {
                        DeadlineSettlement::Active(game)
                        | DeadlineSettlement::Terminal { game, .. } => game,
                    };
                    let terminal = if checked.finished {
                        checked
                    } else {
                        checked
                            .finish_game_control(
                                GameControl::Resign(color),
                                GameResult::Winner(color.opposite_color()),
                                Conclusion::Withdrawal,
                                withdrawn_at,
                                conn,
                            )
                            .await?
                    };
                    tournament_changed = true;
                    replace_game(state, terminal.clone());
                    match parsed_conclusion(&terminal)? {
                        Conclusion::Withdrawal => {}
                        Conclusion::Timeout => {
                            tournament_changed |=
                                seal_terminal_for_batch(state, game_id, withdrawn_at, conn).await?;
                            if matches!(
                                format,
                                Format::SingleElimination | Format::DoubleElimination
                            ) {
                                let progressed =
                                    progress_elimination(state, withdrawn_at, conn).await?;
                                tournament_changed |= progressed.sealed
                                    || progressed.advanced
                                    || !progressed.released_games.is_empty()
                                    || progressed.finished_now;
                                terminal_games.push(terminal);
                                if progressed.finished_now {
                                    return Ok(WithdrawalOutcome {
                                        applied: false,
                                        terminal_games,
                                        commit: Some(fixed_field_commit(state, true)),
                                    });
                                }
                                restart_after_terminal_race = true;
                                break;
                            }
                        }
                        _ => {}
                    }
                    terminal_games.push(terminal);
                }
                WithdrawalObligation::Unstarted { slot_id, game_id } => {
                    let slot_index = state.slot_index(slot_id)?;
                    let result = withdrawal_game_result(&state.slots[slot_index], player_id)?;
                    let game = state.game(game_id)?.clone();
                    let terminal = game
                        .adjudicate_unstarted(&result, Conclusion::Withdrawal, withdrawn_at, conn)
                        .await?;
                    replace_game(state, terminal.clone());
                    tournament_changed = true;
                    terminal_games.push(terminal);
                }
                WithdrawalObligation::Unreleased => {}
            }
        }
        if restart_after_terminal_race {
            continue 'normalize;
        }
        break;
    }

    let transitions = withdrawal_slot_updates(state, player_id)?;
    let applied =
        apply_withdrawal(state, player, player_id, &transitions, withdrawn_at, conn).await?;
    if !applied {
        let changed = tournament_changed
            || !terminal_games.is_empty()
            || !state.schedule_offer_updates.is_empty();
        return Ok(WithdrawalOutcome {
            applied: false,
            terminal_games,
            commit: changed.then(|| fixed_field_commit(state, false)),
        });
    }
    if matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    ) {
        record_elimination_withdrawal(state, player, conn).await?;
    }
    let progressed = progress_after_withdrawal(state, format, withdrawn_at, conn).await?;
    Ok(WithdrawalOutcome {
        applied: true,
        terminal_games,
        commit: Some(fixed_field_commit(state, progressed.finished_now)),
    })
}

async fn progress_after_withdrawal(
    state: &mut TournamentState,
    format: Format,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    match format {
        Format::RoundRobin => {
            let finished_now = finish_automatically_if_required(state, progressed_at, conn).await?;
            Ok(ProgressionEffects {
                finished_now,
                ..ProgressionEffects::default()
            })
        }
        Format::Swiss | Format::DoubleSwiss => progress_swiss(state, progressed_at, conn).await,
        Format::SingleElimination | Format::DoubleElimination => {
            progress_elimination(state, progressed_at, conn).await
        }
        _ => Err(invalid_input(
            "Withdrawal progression received an unsupported tournament format",
        )),
    }
}

fn parsed_conclusion(game: &Game) -> Result<Conclusion, DbError> {
    Conclusion::from_str(&game.conclusion).map_err(|error| {
        invalid_persisted(&format!("invalid Hive tournament game conclusion: {error}"))
    })
}

fn replace_game(state: &mut TournamentState, replacement: Game) {
    if let Some(game) = state
        .games
        .iter_mut()
        .find(|game| game.id == replacement.id)
    {
        *game = replacement;
    }
}

fn withdrawal_slot_updates(
    state: &TournamentState,
    player_id: Uuid,
) -> Result<Vec<SlotResolutionUpdate>, DbError> {
    let games_by_slot = state.games_by_slot();
    state
        .slots
        .iter()
        .filter(|slot| {
            (slot.white == player_id || slot.black == player_id) && slot.resolution.is_none()
        })
        .map(|slot| {
            Ok(SlotResolutionUpdate {
                slot_id: slot.id,
                resolution: Resolution::Withdrawal(withdrawal_resolution_outcome(
                    games_by_slot.get(&slot.id).copied(),
                    slot,
                    player_id,
                )?),
            })
        })
        .collect()
}

fn withdrawal_resolution_outcome(
    game: Option<&Game>,
    slot: &TournamentSlot,
    player_id: Uuid,
) -> Result<GameOutcome, DbError> {
    if let Some(game) = game {
        if game.finished {
            return terminal_game_outcome(game);
        }
    }
    withdrawal_outcome(slot, player_id)
}

fn withdrawal_outcome(slot: &TournamentSlot, player_id: Uuid) -> Result<GameOutcome, DbError> {
    let (white, black) = if slot.white == player_id {
        (
            AdjudicatedSideResult::ForfeitLoss,
            AdjudicatedSideResult::ForfeitWin,
        )
    } else {
        (
            AdjudicatedSideResult::ForfeitWin,
            AdjudicatedSideResult::ForfeitLoss,
        )
    };
    AdjudicatedGameOutcome::new(white, black)
        .map(GameOutcome::Adjudicated)
        .map_err(|error| invalid_input(&format!("Invalid withdrawal outcome: {error}")))
}

fn withdrawal_game_result(
    slot: &TournamentSlot,
    player_id: Uuid,
) -> Result<TournamentGameResult, DbError> {
    if slot.white == player_id {
        Ok(TournamentGameResult::Winner(Color::Black))
    } else {
        Ok(TournamentGameResult::Winner(Color::White))
    }
}

fn withdrawal_action_phase(action: &WithdrawalObligation) -> u8 {
    match action {
        WithdrawalObligation::Terminal { .. } => 0,
        WithdrawalObligation::Played { .. } => 1,
        WithdrawalObligation::Unstarted { .. } => 2,
        WithdrawalObligation::Unreleased => 2,
    }
}

fn withdrawal_instant(state: &TournamentState, requested_at: DateTime<Utc>) -> DateTime<Utc> {
    let latest_acceptance = state
        .memberships
        .iter()
        .map(|membership| membership.accepted_at)
        .max()
        .unwrap_or(requested_at);
    requested_at.max(latest_acceptance)
}
