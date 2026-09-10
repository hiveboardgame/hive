use crate::{
    db_error::DbError,
    models::{
        Game,
        NewGame,
        ScheduleOffer,
        Tournament,
        TournamentFinalOutcome,
        TournamentSlot,
        TournamentSlotInsert,
        TournamentUser,
    },
    schema::{games, tournament_slots, tournaments},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{dsl::sql, prelude::*, sql_types::Text};
use diesel_async::RunQueryDsl;
use hive_lib::{Color, GameResult, GameStatus};
use shared_types::{
    tournament::{
        standings::Snapshot,
        AdjudicatedGameOutcome,
        AdjudicatedSideResult,
        Format,
        GameOutcome,
        PlayedGameOutcome,
        Resolution,
        Slot,
        SlotKey,
    },
    GameStart,
    TournamentGameResult,
};
use std::{collections::HashSet, str::FromStr};
use tournamint::PlayerId;
use uuid::Uuid;

use super::{
    state::{invalid_persisted, TournamentState},
    SlotSealedEvent,
};

pub(crate) async fn release_slot(
    state: &mut TournamentState,
    slot_id: Uuid,
    released_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Game, DbError> {
    let index = state.slot_index(slot_id)?;
    let already_released = state.slot_game(slot_id).is_some();
    if state.slots[index].resolution.is_none() && !already_released {
        ensure_slot_entrants_active(state, &state.slots[index])?;
    }
    let slot = state.slots[index].as_slot();
    let game = release_slot_into_games(
        state.tournament.id,
        &slot,
        &mut state.games,
        &state.bot_user_ids,
        released_at,
        conn,
    )
    .await?;
    if game.game_status == GameStatus::InProgress.to_string() {
        state.schedule_offer_updates.extend(
            ScheduleOffer::close_pending_for_slot(state.tournament.id, slot_id, released_at, conn)
                .await?,
        );
    }
    if !already_released {
        state.record_slot_release(slot_id, &game);
    }
    Ok(game)
}

fn ensure_slot_entrants_active(
    state: &TournamentState,
    slot: &TournamentSlot,
) -> Result<(), DbError> {
    let withdrawn = state
        .memberships
        .iter()
        .filter(|membership| membership.withdrawn_at.is_some())
        .map(|membership| membership.user_id)
        .collect::<HashSet<_>>();
    if [slot.white, slot.black]
        .into_iter()
        .any(|entrant| withdrawn.contains(&entrant))
    {
        return Err(DbError::InvalidAction {
            info: String::from("A withdrawn entrant cannot receive a new tournament game"),
        });
    }
    Ok(())
}

pub(super) async fn release_slot_into_games(
    tournament_id: Uuid,
    slot: &Slot,
    games: &mut Vec<Game>,
    bot_user_ids: &HashSet<Uuid>,
    released_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Game, DbError> {
    if let Some(game) = games
        .iter()
        .find(|game| game.tournament_slot_id == Some(slot.id))
    {
        return Ok(game.clone());
    }
    if slot.resolution.is_some() {
        return Err(DbError::InvalidAction {
            info: format!("Tournament slot {} is already terminal", slot.id),
        });
    }

    let mut game = Game::create(
        NewGame::for_tournament_slot(tournament_id, slot, released_at)?,
        conn,
    )
    .await?;
    if game.game_start == GameStart::Ready.to_string()
        && bot_user_ids.contains(&game.white_id)
        && bot_user_ids.contains(&game.black_id)
    {
        game = game.start(released_at, conn).await?;
    }
    games.push(game.clone());
    Ok(game)
}

pub(crate) async fn materialize_slots(
    state: &mut TournamentState,
    new_slots: Vec<TournamentSlotInsert>,
    release: Vec<SlotKey>,
    effective_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>, DbError> {
    let materialized =
        TournamentSlot::insert_many(state.tournament.id, &new_slots, effective_at, conn).await?;
    let release = release.into_iter().collect::<HashSet<_>>();
    let release_ids = materialized
        .iter()
        .filter(|slot| release.contains(&slot.key))
        .map(|slot| slot.id)
        .collect::<Vec<_>>();
    state.record_materialized_slots(&materialized);
    state.slots.extend(materialized);
    let mut released = Vec::with_capacity(release.len());
    for slot_id in release_ids {
        released.push(release_slot(state, slot_id, effective_at, conn).await?);
    }
    Ok(released)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SlotResolutionUpdate {
    pub(crate) slot_id: Uuid,
    pub(crate) resolution: Resolution,
}

pub(crate) async fn apply_slot_resolutions(
    state: &mut TournamentState,
    updates: &[SlotResolutionUpdate],
    resolved_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    for update in updates {
        let index = state.slot_index(update.slot_id)?;
        if state.slots[index].resolution.is_some() {
            return Err(DbError::InvalidAction {
                info: format!("Tournament slot {} is already terminal", update.slot_id),
            });
        }
        state.slots[index].resolution = Some(update.resolution);
        state.schedule_offer_updates.extend(
            TournamentSlot::persist_resolution(
                state.tournament.id,
                &state.slots[index],
                resolved_at,
                conn,
            )
            .await?,
        );
        state.record_slot_resolution_change(update.slot_id, false, true);
    }
    Ok(())
}

pub(crate) async fn apply_withdrawal(
    state: &mut TournamentState,
    player: PlayerId,
    user_id: Uuid,
    updates: &[SlotResolutionUpdate],
    withdrawn_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    let index = player.index();
    if state.memberships[index].withdrawn_at.is_some() {
        return Ok(false);
    }
    let membership =
        TournamentUser::persist_withdrawal(state.tournament.id, user_id, withdrawn_at, conn)
            .await?;
    state.memberships[index] = membership;
    state.record_membership_change(user_id);
    apply_slot_resolutions(state, updates, withdrawn_at, conn).await?;
    Ok(true)
}

pub(crate) async fn seal_slot(
    state: &mut TournamentState,
    event: SlotSealedEvent,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    seal_slot_with_cleanup_at(state, event, event.sealed_at, conn).await
}

pub(crate) async fn seal_slot_with_cleanup_at(
    state: &mut TournamentState,
    event: SlotSealedEvent,
    schedule_cleanup_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    let index = state.slot_index(event.slot_id)?;
    if state.slots[index].resolution.is_some() {
        return Ok(false);
    }
    state.slots[index].resolution = Some(Resolution::Result(event.outcome));
    state.schedule_offer_updates.extend(
        TournamentSlot::persist_resolution_with_cleanup_at(
            state.tournament.id,
            &state.slots[index],
            event.sealed_at,
            schedule_cleanup_at,
            conn,
        )
        .await?,
    );
    state.record_slot_resolution_change(event.slot_id, false, true);
    Ok(true)
}

pub(crate) async fn correct_slot(
    state: &mut TournamentState,
    slot_id: Uuid,
    replacement: Option<GameOutcome>,
    game_replacement: Option<TournamentGameResult>,
    corrected_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    if state.configuration.format() == Format::Arena {
        return Err(DbError::InvalidAction {
            info: String::from("Arena results are not correctable"),
        });
    }
    let index = state.slot_index(slot_id)?;
    let current = state.slots[index]
        .resolution
        .ok_or_else(|| DbError::InvalidAction {
            info: String::from("Only a resolved tournament slot can be corrected"),
        })?;
    match current {
        Resolution::Result(_) | Resolution::Withdrawal(_) => {}
        Resolution::Clinched => {
            return Err(DbError::InvalidAction {
                info: String::from("A clinched slot cannot be corrected"),
            })
        }
    }
    if matches!(current, Resolution::Withdrawal(_)) {
        return Err(DbError::InvalidAction {
            info: String::from("A withdrawal resolution cannot be corrected"),
        });
    }
    let game_index = state
        .games
        .iter()
        .position(|game| Some(slot_id) == game.tournament_slot_id);
    if replacement.is_none() {
        ensure_slot_entrants_active(state, &state.slots[index])?;
    }
    state.slots[index].resolution = replacement.map(Resolution::Result);
    if let Some(game_index) = game_index {
        let game = match game_replacement {
            Some(result) => {
                state.games[game_index]
                    .replace_adjudication(&result, corrected_at, conn)
                    .await?
            }
            None => {
                state.games[game_index]
                    .clear_adjudication(corrected_at, conn)
                    .await?
            }
        };
        state.games[game_index] = game;
    }
    state.schedule_offer_updates.extend(
        TournamentSlot::persist_resolution(
            state.tournament.id,
            &state.slots[index],
            corrected_at,
            conn,
        )
        .await?,
    );
    state.record_slot_resolution_change(slot_id, true, replacement.is_some());
    Ok(())
}

pub(crate) async fn persist_finished_with_outcome(
    tournament: &Tournament,
    standings: Snapshot,
    finished_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Tournament, DbError> {
    TournamentFinalOutcome::insert(tournament.id, standings, conn).await?;
    ScheduleOffer::purge_tournament(tournament.id, conn).await?;
    let tournament = Tournament::persist_finished(tournament, finished_at, conn).await?;
    Ok(tournament)
}

pub(crate) async fn expired_fixed_field_game_ids(
    formats: &[Format],
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    if limit <= 0 || formats.is_empty() {
        return Ok(Vec::new());
    }
    let families = formats
        .iter()
        .map(|format| format.family_tag())
        .collect::<Vec<_>>();
    Ok(games::table
        .inner_join(tournaments::table.on(games::tournament_id.eq(tournaments::id.nullable())))
        .filter(games::tournament_slot_id.is_not_null())
        .filter(games::finished.eq(false))
        .filter(games::timeout_at.is_not_null())
        .filter(games::timeout_at.le(as_of))
        .filter(tournaments::started_at.is_not_null())
        .filter(tournaments::finished_at.is_null())
        .filter(sql::<Text>("configuration #>> '{format,format}'").eq_any(families))
        .order((games::timeout_at, games::id))
        .limit(limit)
        .select(games::id)
        .load(conn)
        .await?)
}

pub(crate) async fn fixed_field_reconciliation_candidate_ids(
    formats: &[Format],
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    if limit <= 0 || formats.is_empty() {
        return Ok(Vec::new());
    }
    let families = formats
        .iter()
        .map(|format| format.family_tag())
        .collect::<Vec<_>>();
    Ok(tournament_slots::table
        .inner_join(
            games::table.on(games::tournament_slot_id
                .eq(tournament_slots::id.nullable())
                .and(games::tournament_id.eq(tournament_slots::tournament_id.nullable()))),
        )
        .inner_join(tournaments::table.on(tournament_slots::tournament_id.eq(tournaments::id)))
        .filter(tournament_slots::resolution.is_null())
        .filter(games::finished.eq(true))
        .filter(tournaments::started_at.is_not_null())
        .filter(tournaments::finished_at.is_null())
        .filter(sql::<Text>("configuration #>> '{format,format}'").eq_any(families))
        .select(tournament_slots::tournament_id)
        .distinct()
        .order(tournament_slots::tournament_id)
        .limit(limit)
        .load(conn)
        .await?)
}

pub(crate) async fn reconciliation_slot_ids(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    Ok(tournament_slots::table
        .filter(tournament_slots::tournament_id.eq(tournament_id))
        .select(tournament_slots::id)
        .order(tournament_slots::id)
        .load(conn)
        .await?)
}

pub(crate) fn terminal_game_outcome(game: &Game) -> Result<GameOutcome, DbError> {
    let result = TournamentGameResult::from_str(&game.tournament_game_result).map_err(|error| {
        invalid_persisted(&format!("invalid Hive tournament game result: {error}"))
    })?;
    let status = GameStatus::from_str(&game.game_status)
        .map_err(|error| invalid_persisted(&format!("invalid game status: {error}")))?;
    normalize_terminal_game_outcome(status, result)
}

fn normalize_terminal_game_outcome(
    status: GameStatus,
    result: TournamentGameResult,
) -> Result<GameOutcome, DbError> {
    match status {
        GameStatus::Finished(game_result) => {
            let outcome = match game_result {
                GameResult::Winner(Color::White) => PlayedGameOutcome::WhiteWin,
                GameResult::Winner(Color::Black) => PlayedGameOutcome::BlackWin,
                GameResult::Draw => PlayedGameOutcome::Draw,
                GameResult::Unknown => {
                    return Err(invalid_persisted("a terminal game has no played outcome"))
                }
            };
            Ok(GameOutcome::Played(outcome))
        }
        GameStatus::Adjudicated => {
            let sides = match result {
                TournamentGameResult::Winner(Color::White) => (
                    AdjudicatedSideResult::ForfeitWin,
                    AdjudicatedSideResult::ForfeitLoss,
                ),
                TournamentGameResult::Winner(Color::Black) => (
                    AdjudicatedSideResult::ForfeitLoss,
                    AdjudicatedSideResult::ForfeitWin,
                ),
                TournamentGameResult::Draw => {
                    (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw)
                }
                TournamentGameResult::DoubleForfeit => (
                    AdjudicatedSideResult::DoubleForfeit,
                    AdjudicatedSideResult::DoubleForfeit,
                ),
                TournamentGameResult::Unknown => {
                    return Err(invalid_persisted(
                        "an adjudicated game has no tournament outcome",
                    ))
                }
            };
            AdjudicatedGameOutcome::new(sides.0, sides.1)
                .map(GameOutcome::Adjudicated)
                .map_err(|error| invalid_persisted(&error.to_string()))
        }
        GameStatus::NotStarted | GameStatus::InProgress => Err(invalid_persisted(
            "a terminal game has a nonterminal status",
        )),
    }
}
