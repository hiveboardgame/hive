use crate::{
    db_error::DbError,
    models::{
        ArenaGameResult,
        FrozenArenaRating,
        Game,
        Rating,
        Tournament,
        TournamentFinalOutcome,
        TournamentUser,
    },
    schema::{ratings, users},
    tournaments::rating,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, result::Error as DieselError};
use diesel_async::{
    AnsiTransactionManager,
    AsyncConnection,
    RunQueryDsl,
    SimpleAsyncConnection,
    TransactionManager,
};
use shared_types::{tournament::arena::PairingIntent, GameSpeed, TournamentStatus};
use std::str::FromStr;
use tournamint::arena::{self, ArenaConfig as TournamintArenaConfig};
use uuid::Uuid;

use super::{
    super::{arena::FinalizeOutcome as ArenaFinalizeOutcome, state::invalid_persisted},
    projection::finished_snapshot,
    state::{
        arena_configuration,
        awarded_facts,
        load_arena_finalization_snapshot,
        load_arena_scoring_games,
        projection_players,
        terminal_facts,
        terminal_facts_before,
        ArenaDbState,
    },
};

pub(crate) async fn settle_arena_game(
    tournament_id: Uuid,
    game: &Game,
    terminal_at: DateTime<Utc>,
    requeue_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    if !game.finished
        || game.tournament_id != Some(tournament_id)
        || game.tournament_slot_id.is_some()
        || game.arena_ordinal.is_none()
        || game.finished_at.map(|at| at.timestamp_micros()) != Some(terminal_at.timestamp_micros())
    {
        return Err(invalid_persisted(
            "Arena settlement requires a terminal Arena-owned Game",
        ));
    }
    // A single snapshot retains both players' ordered results and awards.
    // Their prior games are committed before either can be paired again.
    let games =
        load_arena_scoring_games(tournament_id, [game.white_id, game.black_id], conn).await?;
    let terminals = terminal_facts(&games)?;
    let index = terminals
        .iter()
        .position(|terminal| terminal.game.game.id == game.id)
        .ok_or_else(|| {
            invalid_persisted("New Arena terminal is missing from its scoring snapshot")
        })?;
    for (position, terminal) in terminals.iter().enumerate() {
        if terminal.game.award.is_none() != (position == index) {
            return Err(invalid_persisted(
                "Arena settlement requires exactly one unawarded terminal",
            ));
        }
    }
    let prior = terminals
        .iter()
        .take(index)
        .map(|terminal| terminal.native)
        .collect::<Vec<_>>();
    let award = arena::award_game(
        &TournamintArenaConfig::default(),
        &prior,
        &terminals[index].native,
    )
    .map_err(|error| invalid_persisted(&format!("Arena scoring facts are invalid: {error}")))?;
    ArenaGameResult::insert(tournament_id, game.id, award, conn).await?;
    persist_completed_game_players(tournament_id, game, requeue_at, conn).await
}

async fn persist_completed_game_players(
    tournament_id: Uuid,
    game: &Game,
    requeue_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    let speed = GameSpeed::from_str(&game.speed)
        .map_err(|error| invalid_persisted(&format!("Arena Game has invalid speed: {error}")))?;
    let no_start_absent = if game.is_arena_no_start() {
        Some(if game.turn == 0 {
            game.white_id
        } else {
            game.black_id
        })
    } else {
        None
    };
    let user_ids = [
        game.white_id.min(game.black_id),
        game.white_id.max(game.black_id),
    ];
    let memberships =
        TournamentUser::find_by_user_ids_for_update(tournament_id, &user_ids, conn).await?;
    if memberships.len() != user_ids.len() {
        return Err(invalid_persisted(
            "Arena Game players are not both tournament participants",
        ));
    }
    for membership in memberships {
        let user_id = membership.user_id;
        let current_intent = membership
            .pairing_intent()?
            .ok_or_else(|| invalid_persisted("Arena participant has no pairing intent"))?;
        let (intent, waiting_since) = if no_start_absent == Some(user_id) {
            (PairingIntent::Paused, None)
        } else if current_intent == PairingIntent::Enabled {
            (current_intent, Some(requeue_at))
        } else {
            (current_intent, None)
        };
        let arena_rating = rating::rounded(Rating::for_uuid(&user_id, &speed, conn).await?.rating)
            .ok_or_else(|| invalid_persisted("Arena participant has an invalid rating"))?;
        TournamentUser::persist_arena_completion_state(
            tournament_id,
            user_id,
            arena_rating,
            Some(intent),
            waiting_since,
            conn,
        )
        .await?;
    }
    Ok(())
}

/// Owns a top-level READ COMMITTED transaction. The roster lock must be acquired
/// before the final facts' statement snapshot; an existing transaction is rejected.
pub async fn finalize_due(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<ArenaFinalizeOutcome, DbError> {
    finalize_due_with_clock(tournament_id, Utc::now, conn).await
}

pub(super) async fn finalize_due_with_clock<F>(
    tournament_id: Uuid,
    now: F,
    conn: &mut DbConn<'_>,
) -> Result<ArenaFinalizeOutcome, DbError>
where
    F: FnOnce() -> DateTime<Utc> + Send,
{
    if AnsiTransactionManager::transaction_manager_status_mut(&mut **conn)
        .transaction_depth()?
        .is_some()
    {
        return Err(DieselError::AlreadyInTransaction.into());
    }
    conn.transaction::<_, DbError, _>(async move |tc| {
        tc.batch_execute("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .await?;
        finalize_due_in_transaction_with_clock(tournament_id, now, tc).await
    })
    .await
}

/// Finalizes inside a transaction owned by the caller, which must roll back on
/// error and retain the tournament lock through commit. Use READ COMMITTED for
/// shared tournaments. Stronger isolation is suitable only when the caller owns
/// a newly created, unpublished tournament, as the synthetic importer does.
pub async fn finalize_due_in_transaction(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<ArenaFinalizeOutcome, DbError> {
    finalize_due_in_transaction_with_clock(tournament_id, Utc::now, conn).await
}

async fn finalize_due_in_transaction_with_clock<F>(
    tournament_id: Uuid,
    now: F,
    conn: &mut DbConn<'_>,
) -> Result<ArenaFinalizeOutcome, DbError>
where
    F: FnOnce() -> DateTime<Utc> + Send,
{
    if AnsiTransactionManager::transaction_manager_status_mut(&mut **conn)
        .transaction_depth()?
        .is_none()
    {
        return Err(DieselError::NotInTransaction.into());
    }
    let tournament = Tournament::find_for_update(tournament_id, conn).await?;
    let observed_at = now();
    let configuration = arena_configuration(&tournament)?.clone();
    match tournament.status() {
        TournamentStatus::NotStarted => Err(DbError::InvalidAction {
            info: String::from("Arena tournament has not started"),
        }),
        TournamentStatus::Finished => Ok(ArenaFinalizeOutcome {
            tournament,
            finished_now: false,
        }),
        TournamentStatus::InProgress => {
            let state = load_arena_finalization_snapshot(tournament, configuration, conn).await?;
            let ends_at = state.boundaries.ends_at;
            if observed_at < ends_at {
                return Err(DbError::InvalidAction {
                    info: String::from("The Arena scoring cutoff is not due"),
                });
            }
            let tournament = finish_arena(&state, ends_at, observed_at, conn).await?;
            Ok(ArenaFinalizeOutcome {
                tournament,
                finished_now: true,
            })
        }
    }
}

async fn finish_arena(
    state: &ArenaDbState,
    ends_at: DateTime<Utc>,
    observed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Tournament, DbError> {
    let players = projection_players(state);
    let terminals = terminal_facts_before(&state.games, ends_at)?;
    let awarded = awarded_facts(&terminals)?;
    let projection = arena::project(&TournamintArenaConfig::default(), &players, &awarded)
        .map_err(|e| invalid_persisted(&format!("Arena facts are invalid: {e}")))?;
    let ratings = state
        .memberships
        .iter()
        .map(|membership| FrozenArenaRating {
            user_id: membership.user_id,
            rating: membership.rating,
        })
        .collect();
    let included_game_ids = terminals
        .iter()
        .map(|terminal| terminal.game.game.id)
        .collect::<Vec<_>>();
    TournamentFinalOutcome::insert_arena(
        state.tournament.id,
        finished_snapshot(state, &projection),
        ratings,
        &included_game_ids,
        conn,
    )
    .await?;
    Tournament::persist_finished(&state.tournament, observed_at, conn).await
}

pub(super) async fn sample_arena_ratings(
    user_ids: &[Uuid],
    speed: GameSpeed,
    conn: &mut DbConn<'_>,
) -> Result<Vec<(Uuid, i32)>, DbError> {
    let speed = speed.to_string();
    users::table
        .left_join(
            ratings::table.on(ratings::user_uid
                .eq(users::id)
                .and(ratings::speed.eq(speed))),
        )
        .filter(users::id.eq_any(user_ids))
        .order(users::id)
        .select((users::id, ratings::rating.nullable().assume_not_null()))
        .load::<(Uuid, f64)>(conn)
        .await?
        .into_iter()
        .map(|(user_id, value)| {
            rating::rounded(value)
                .map(|rating| (user_id, rating))
                .ok_or_else(|| invalid_persisted("Arena participant has an invalid rating"))
        })
        .collect::<Result<Vec<_>, DbError>>()
}
