use crate::{
    db_error::DbError,
    models::{DeadlineSettlement, Game, Rating, Tournament, TournamentSlot, TournamentUser, User},
    tournaments::{fixed_field::WithdrawalOutcome, TournamentGame},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel_async::AsyncConnection;
use hive_lib::{Color, GameControl, GameResult};
use shared_types::{
    tournament::{
        AdjudicatedGameOutcome,
        AdjudicatedSideResult,
        Format,
        GameOutcome,
        Resolution,
        SlotKey,
    },
    Conclusion,
    TournamentGameResult,
};
use std::str::FromStr;
use tournamint::PlayerId;
use uuid::Uuid;

use super::{
    super::{
        apply_withdrawal,
        elimination::settle_elimination_series_before_withdrawal,
        progress_elimination,
        progress_swiss,
        reconciliation_slot_ids,
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
    let (_, mutable_game_ids) = withdrawal_lock_ids(&state, player_id);
    let locked_games = Game::find_by_ids_for_update(&mutable_game_ids, conn).await?;
    for game in locked_games {
        if let Some(current) = state.games.iter_mut().find(|current| current.id == game.id) {
            *current = (&game).into();
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
            *current = game.into();
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
        let slot_ids = reconciliation_slot_ids(tournament_id, tc).await?;
        TournamentSlot::find_by_ids_for_update(tournament_id, &slot_ids, tc).await?;
        let state = load_in_progress(tournament, BotUsers::ForGameRelease, tc).await?;
        let mut state = lock_withdrawal_state(state, player_id, tc).await?;
        ensure_adapted_format(&state)?;
        let requested_at = Utc::now();
        let withdrawn_at = withdrawal_instant(&state, requested_at);
        apply_withdrawal_command(
            &mut state,
            player_id,
            withdrawn_at,
            WithdrawalOrigin::UserCommand,
            tc,
        )
        .await
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
    {
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
            let ids = played_resignation_games
                .iter()
                .map(|game| game.id)
                .collect::<Vec<_>>();
            let games = Game::find_by_ids_for_update(&ids, conn).await?;
            Rating::lock_for_game_updates(&games, conn).await?;
        }
        for action in actions {
            match action {
                WithdrawalObligation::Terminal { game_id, .. } => {
                    let game = state.game(game_id)?.clone();
                    let conclusion = parsed_conclusion(&game.conclusion)?;
                    if conclusion != Conclusion::Withdrawal {
                        let terminal_at = game.finished_at.ok_or_else(|| {
                            invalid_persisted("a terminal game has no finished_at")
                        })?;
                        tournament_changed |=
                            seal_terminal_for_batch(state, game_id, terminal_at, conn).await?;
                        if matches!(
                            format,
                            Format::SingleElimination | Format::DoubleElimination
                        ) {
                            settle_elimination_series_before_withdrawal(state, withdrawn_at, conn)
                                .await?;
                        }
                    }
                }
                WithdrawalObligation::Played { game_id, color, .. } => {
                    let game = Game::find_by_uuid(&game_id, conn).await?;
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
                    match parsed_conclusion(&terminal.conclusion)? {
                        Conclusion::Withdrawal => {}
                        Conclusion::Timeout => {
                            let terminal_at = terminal.finished_at.ok_or_else(|| {
                                invalid_persisted("a terminal game has no finished_at")
                            })?;
                            tournament_changed |=
                                seal_terminal_for_batch(state, game_id, terminal_at, conn).await?;
                            if matches!(
                                format,
                                Format::SingleElimination | Format::DoubleElimination
                            ) {
                                settle_elimination_series_before_withdrawal(
                                    state,
                                    withdrawn_at,
                                    conn,
                                )
                                .await?;
                            }
                        }
                        _ => {}
                    }
                    terminal_games.push(terminal);
                }
                WithdrawalObligation::Unstarted { slot_id, game_id } => {
                    let slot_index = state.slot_index(slot_id)?;
                    let result = withdrawal_game_result(&state.slots[slot_index], player_id)?;
                    let game = Game::find_by_uuid(&game_id, conn).await?;
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
    }

    if finish_automatically_if_required(state, withdrawn_at, conn).await? {
        return Ok(WithdrawalOutcome {
            applied: false,
            terminal_games,
            commit: Some(fixed_field_commit(state, true)),
        });
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

fn parsed_conclusion(conclusion: &str) -> Result<Conclusion, DbError> {
    Conclusion::from_str(conclusion).map_err(|error| {
        invalid_persisted(&format!("invalid Hive tournament game conclusion: {error}"))
    })
}

fn replace_game(state: &mut TournamentState, replacement: Game) {
    if let Some(game) = state
        .games
        .iter_mut()
        .find(|game| game.id == replacement.id)
    {
        *game = (&replacement).into();
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
                resolution: if matches!(slot.key, SlotKey::Elimination { .. })
                    && !games_by_slot.contains_key(&slot.id)
                {
                    Resolution::CancelledByWithdrawal
                } else {
                    Resolution::Withdrawal(withdrawal_resolution_outcome(
                        games_by_slot.get(&slot.id).copied(),
                        slot,
                        player_id,
                    )?)
                },
            })
        })
        .collect()
}

fn withdrawal_resolution_outcome(
    game: Option<&TournamentGame>,
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

pub(crate) fn withdrawal_instant(
    state: &TournamentState,
    requested_at: DateTime<Utc>,
) -> DateTime<Utc> {
    let latest_acceptance = state
        .memberships
        .iter()
        .map(|membership| membership.accepted_at)
        .max()
        .unwrap_or(requested_at);
    requested_at.max(latest_acceptance)
}
