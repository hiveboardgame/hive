use crate::{
    responses::{
        DailyArchiveRow,
        DailyPuzzleResponse,
        DailyScheduleResponse,
        PuzzleAttemptResponse,
        PuzzleHistoryRow,
        PuzzleRatingResponse,
        PuzzleResponse,
        PuzzleSetResponse,
        PuzzleVoteResponse,
    },
    security::csrf::CsrfClient,
};
use leptos::prelude::*;
use shared_types::{PuzzleDifficulty, PuzzleGameTypeFilter, PuzzleHistoryFilter, PuzzleLength};

/// Where guests are served from: gentle, mostly short wins, rather than the middle of the pool.
#[cfg(feature = "ssr")]
const GUEST_RATING: f64 = 1150.0;

#[cfg(feature = "ssr")]
async fn viewer() -> Option<uuid::Uuid> {
    crate::functions::auth::identity::uuid().await.ok()
}

#[cfg(feature = "ssr")]
async fn puzzle_rating_of(
    user: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<db_lib::models::Rating, ServerFnError> {
    use db_lib::models::Rating;
    use shared_types::GameSpeed;
    Ok(Rating::for_uuid(&user, &GameSpeed::Puzzle, conn).await?)
}

#[cfg(feature = "ssr")]
async fn respond(
    puzzle: &db_lib::models::Puzzle,
    user: Option<uuid::Uuid>,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<PuzzleResponse, ServerFnError> {
    use db_lib::models::{DailyPuzzle, Puzzle};
    let daily_day = DailyPuzzle::day_of(puzzle.id, conn).await?;
    let (my_vote, attempted) = match user {
        Some(user) => (
            Puzzle::vote_of(puzzle.id, user, conn).await?,
            Puzzle::attempted(puzzle.id, user, conn).await?,
        ),
        None => (None, false),
    };
    Ok(PuzzleResponse::from_model(
        puzzle, my_vote, attempted, daily_day,
    ))
}

/// The next unseen puzzle near the viewer's puzzle rating (1500 for guests). `exclude` is a
/// comma-separated list of nanoids a guest's browser already showed (a string, because an empty
/// list does not survive URL encoding).
#[server(client = CsrfClient)]
pub async fn next_puzzle(
    game_type: PuzzleGameTypeFilter,
    difficulty: PuzzleDifficulty,
    length: PuzzleLength,
    theme: String,
    collection: String,
    exclude: String,
) -> Result<Option<PuzzleResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = viewer().await;
    let around = match user {
        Some(user) => puzzle_rating_of(user, &mut conn).await?.rating,
        None => GUEST_RATING,
    };
    let wanted: Vec<&str> = exclude
        .split(',')
        .filter(|id| !id.is_empty())
        .take(200)
        .collect();
    let excluded = Puzzle::ids_for_nanoids(&wanted, &mut conn).await?;
    use shared_types::PuzzleSelection;
    let selection = PuzzleSelection {
        game_type,
        difficulty,
        length: if theme.starts_with("winIn") {
            PuzzleLength::Any
        } else {
            length
        },
        theme: (!theme.is_empty()).then_some(theme),
        collection: (!collection.is_empty()).then_some(collection),
    };
    let Some(puzzle) = Puzzle::next(around, user, &selection, &excluded, &mut conn).await? else {
        return Ok(None);
    };
    Ok(Some(respond(&puzzle, user, &mut conn).await?))
}

#[server(client = CsrfClient)]
pub async fn get_puzzle(nanoid: String) -> Result<PuzzleResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    respond(&puzzle, viewer().await, &mut conn).await
}

/// Records how a try went. Signed-in first tries are rated; guests only feed the play counts.
#[server(client = CsrfClient)]
pub async fn submit_puzzle_attempt(
    nanoid: String,
    solved: bool,
    hinted: bool,
    time_ms: Option<i32>,
    first_try: bool,
) -> Result<PuzzleAttemptResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    match viewer().await {
        Some(user) => {
            let outcome =
                Puzzle::record_attempt(puzzle.id, user, solved, hinted, time_ms, &mut conn).await?;
            Ok(PuzzleAttemptResponse {
                rated: outcome.rated,
                counted_play: outcome.rated,
                rating_change: outcome
                    .user_before
                    .zip(outcome.user_after)
                    .map(|(before, after)| (after - before).round() as i32),
                rating_before: outcome.user_before.map(|r| r.round() as i32),
                rating_after: outcome.user_after.map(|r| r.round() as i32),
                puzzle_rating_after: outcome.puzzle_after.round() as i32,
            })
        }
        None => {
            if first_try {
                Puzzle::record_guest_play(puzzle.id, solved && !hinted, &mut conn).await?;
            }
            Ok(PuzzleAttemptResponse {
                rated: false,
                counted_play: first_try,
                rating_change: None,
                rating_before: None,
                rating_after: None,
                puzzle_rating_after: puzzle.rating.round() as i32,
            })
        }
    }
}

#[server(client = CsrfClient)]
pub async fn vote_puzzle(
    nanoid: String,
    vote: Option<bool>,
) -> Result<PuzzleVoteResponse, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::Puzzle};
    let user = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    let updated = Puzzle::vote(puzzle.id, user, vote, &mut conn).await?;
    Ok(PuzzleVoteResponse {
        my_vote: vote,
        likes: updated.likes,
        dislikes: updated.dislikes,
    })
}

#[server(client = CsrfClient)]
pub async fn puzzle_history(
    filter: PuzzleHistoryFilter,
    page: i64,
) -> Result<Vec<PuzzleHistoryRow>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        get_conn,
        models::{DailyPuzzle, Puzzle},
    };
    let user = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let rows = Puzzle::history(user, filter, page, &mut conn).await?;
    let ids: Vec<uuid::Uuid> = rows.iter().map(|(_, puzzle, _)| puzzle.id).collect();
    let days = DailyPuzzle::days_of(&ids, &mut conn).await?;
    Ok(rows
        .into_iter()
        .map(|(attempt, puzzle, my_vote)| PuzzleHistoryRow {
            nanoid: puzzle.nanoid.clone(),
            game_type: puzzle.game_type.clone(),
            solver_moves: puzzle.solver_moves,
            rating: puzzle.rating.round() as i32,
            solved: attempt.solved,
            hinted: attempt.hinted,
            rating_change: attempt
                .rating_before
                .zip(attempt.rating_after)
                .map(|(before, after)| (after - before).round() as i32),
            played_at: attempt.created_at,
            my_vote,
            themes: puzzle.themes(),
            title: puzzle.title.clone(),
            history: puzzle.history.clone(),
            position: puzzle.position.clone(),
            daily_day: days
                .iter()
                .find(|(id, _)| *id == puzzle.id)
                .map(|(_, day)| *day),
        })
        .collect())
}

/// The viewer's puzzle rating, or None for guests.
#[server(client = CsrfClient)]
pub async fn my_puzzle_rating() -> Result<Option<PuzzleRatingResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use shared_types::Certainty;
    let Some(user) = viewer().await else {
        return Ok(None);
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let rating = puzzle_rating_of(user, &mut conn).await?;
    let streak = db_lib::models::DailyPuzzle::streak(user, today(), &mut conn).await?;
    Ok(Some(PuzzleRatingResponse {
        rating: rating.rating.round() as i32,
        played: rating.played,
        won: rating.won,
        provisional: Certainty::from_deviation(rating.deviation) != Certainty::Rankable,
        streak: crate::responses::PuzzleStreakResponse {
            current: streak.current,
            best: streak.best,
        },
    }))
}

/// Puzzles mined from one game, for linking from its page.
#[server(client = CsrfClient)]
pub async fn puzzles_from_game(nanoid: String) -> Result<Vec<(String, i32, i32)>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    Ok(Puzzle::for_source_game(&nanoid, &mut conn)
        .await?
        .into_iter()
        .map(|puzzle| {
            (
                puzzle.nanoid,
                puzzle.solver_moves,
                puzzle.rating.round() as i32,
            )
        })
        .collect())
}

#[server(client = CsrfClient)]
pub async fn puzzle_sets() -> Result<Vec<PuzzleSetResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = viewer().await;
    Ok(Puzzle::set_counts(user, &mut conn)
        .await?
        .into_iter()
        .map(|count| PuzzleSetResponse {
            key: count.key,
            total: count.total,
            solved: count.solved_count,
        })
        .collect())
}

/// Nodes the refutation search may spend; positions this short need far fewer.
#[cfg(feature = "ssr")]
const REFUTATION_BUDGET: u64 = 3_000_000;
/// How many solver moves ahead the opponent looks while a missed puzzle is played out.
#[cfg(feature = "ssr")]
const DEFENCE_HORIZON: i32 = 2;
#[cfg(feature = "ssr")]
const MAX_PLAYED_OUT_PLIES: usize = 200;

/// After a wrong move: the opponent's answer that leaves no forced win in the moves the puzzle
/// still allowed. `moves` are the moves played since the puzzle's start, `;`-separated.
#[server(client = CsrfClient)]
pub async fn puzzle_refutation(
    nanoid: String,
    moves: String,
) -> Result<Option<String>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    use shared_types::{play_token, start_state};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    let mut state = start_state(
        &puzzle.game_type,
        puzzle.history.as_deref(),
        puzzle.position.as_deref(),
    )
    .map_err(ServerFnError::new)?;
    let tokens: Vec<&str> = moves
        .split(';')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .take(64)
        .collect();
    for token in &tokens {
        play_token(&mut state, token).map_err(ServerFnError::new)?;
    }
    let solver_moves_made = tokens.len().div_ceil(2) as i32;
    let left = (puzzle.solver_moves - solver_moves_made).max(0) as u32;
    let found = tokio::task::spawn_blocking(move || {
        puzzler::refute::refutation(&state, left, REFUTATION_BUDGET)
    })
    .await
    .map_err(ServerFnError::new)?;
    Ok(found.ok().flatten())
}

/// While a missed puzzle is played out: the opponent's stubbornest answer to the moves played
/// since the puzzle's start (`;`-separated), or None when it is not the opponent's turn.
#[server(client = CsrfClient)]
pub async fn puzzle_defence(
    nanoid: String,
    moves: String,
) -> Result<Option<String>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::Puzzle};
    use shared_types::{play_token, start_state};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    let mut state = start_state(
        &puzzle.game_type,
        puzzle.history.as_deref(),
        puzzle.position.as_deref(),
    )
    .map_err(ServerFnError::new)?;
    let solver = state.turn_color;
    let tokens: Vec<&str> = moves
        .split(';')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .take(MAX_PLAYED_OUT_PLIES)
        .collect();
    for token in &tokens {
        play_token(&mut state, token).map_err(ServerFnError::new)?;
    }
    let solver_moves_made = tokens.len().div_ceil(2) as i32;
    let left = (puzzle.solver_moves - solver_moves_made).max(DEFENCE_HORIZON) as u32;
    tokio::task::spawn_blocking(move || {
        puzzler::refute::best_defence(&state, solver, left, REFUTATION_BUDGET)
    })
    .await
    .map_err(ServerFnError::new)
}

#[cfg(feature = "ssr")]
fn today() -> chrono::NaiveDate {
    chrono::Utc::now().date_naive()
}

/// Today's puzzle (UTC), chosen on the first request of the day.
#[server(client = CsrfClient)]
pub async fn daily_puzzle() -> Result<Option<DailyPuzzleResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::DailyPuzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let Some((daily, puzzle)) = DailyPuzzle::for_day(today(), &mut conn).await? else {
        return Ok(None);
    };
    let user = viewer().await;
    let tally = DailyPuzzle::tally(puzzle.id, daily.day, &mut conn).await?;
    let (my_result, streak) = match user {
        Some(user) => {
            let streak = DailyPuzzle::streak(user, today(), &mut conn).await?;
            (
                DailyPuzzle::first_result(puzzle.id, user, &mut conn).await?,
                Some(crate::responses::PuzzleStreakResponse {
                    current: streak.current,
                    best: streak.best,
                }),
            )
        }
        None => (None, None),
    };
    Ok(Some(DailyPuzzleResponse {
        day: daily.day,
        puzzle: respond(&puzzle, user, &mut conn).await?,
        tried: tally.tried,
        solved: tally.solved,
        my_result,
        streak,
    }))
}

/// The day a puzzle was the puzzle of the day, if ever.
#[server(client = CsrfClient)]
pub async fn puzzle_daily_day(nanoid: String) -> Result<Option<chrono::NaiveDate>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{
        get_conn,
        models::{DailyPuzzle, Puzzle},
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    Ok(DailyPuzzle::day_of(puzzle.id, &mut conn).await?)
}

#[server(client = CsrfClient)]
pub async fn daily_schedule() -> Result<DailyScheduleResponse, ServerFnError> {
    use crate::{
        functions::{auth::identity::ensure_admin, db::pool},
        responses::DailyQueueEntry,
    };
    use db_lib::{get_conn, models::DailyPuzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    let today = today();
    let recent = DailyPuzzle::recent(today - chrono::Duration::days(14), &mut conn).await?;
    let first_open = if recent.iter().any(|entry| entry.day >= today) {
        today + chrono::Duration::days(1)
    } else {
        today
    };
    let queued = DailyPuzzle::queued(&mut conn)
        .await?
        .into_iter()
        .zip(0..)
        .map(|(queued, offset)| DailyQueueEntry {
            nanoid: queued.puzzle.nanoid.clone(),
            day: first_open + chrono::Duration::days(offset),
            game_type: queued.puzzle.game_type.clone(),
            solver_moves: queued.puzzle.solver_moves,
            rating: queued.puzzle.rating.round() as i32,
        })
        .collect();
    Ok(DailyScheduleResponse {
        recent: recent
            .into_iter()
            .map(|entry| (entry.day, entry.nanoid))
            .collect(),
        queued,
    })
}

/// Accepts a nanoid or a pasted puzzle link.
#[server(client = CsrfClient)]
pub async fn queue_daily_puzzle(nanoid: String) -> Result<(), ServerFnError> {
    use crate::functions::{
        auth::identity::{ensure_admin, uuid},
        db::pool,
    };
    use db_lib::{get_conn, models::DailyPuzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    let admin = uuid().await?;
    let nanoid = nanoid
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_string();
    DailyPuzzle::enqueue(&nanoid, admin, &mut conn)
        .await
        .map_err(|error| match error {
            db_lib::db_error::DbError::InvalidAction { info } => ServerFnError::new(info),
            db_lib::db_error::DbError::NotFound { .. } => {
                ServerFnError::new(format!("No puzzle {nanoid}."))
            }
            other => ServerFnError::new(other),
        })
}

#[server(client = CsrfClient)]
pub async fn unqueue_daily_puzzle(nanoid: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::{get_conn, models::DailyPuzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    Ok(DailyPuzzle::dequeue(&nanoid, &mut conn).await?)
}

/// Earlier puzzles of the day, newest first, 30 per page.
#[server(client = CsrfClient)]
pub async fn daily_archive(page: i64) -> Result<Vec<DailyArchiveRow>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{get_conn, models::DailyPuzzle};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = viewer().await;
    Ok(DailyPuzzle::archive(today(), user, page, &mut conn)
        .await?
        .into_iter()
        .map(|entry| DailyArchiveRow {
            day: entry.day,
            nanoid: entry.puzzle.nanoid,
            game_type: entry.puzzle.game_type,
            history: entry.puzzle.history,
            position: entry.puzzle.position,
            solver_moves: entry.puzzle.solver_moves,
            rating: entry.puzzle.rating.round() as i32,
            my_result: entry.my_result,
        })
        .collect())
}
