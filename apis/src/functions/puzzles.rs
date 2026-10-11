use crate::{
    responses::{
        PuzzleAttemptResponse,
        PuzzleHistoryRow,
        PuzzleRatingResponse,
        PuzzleResponse,
        PuzzleSetResponse,
        PuzzleStarsResponse,
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
    use db_lib::models::Puzzle;
    let (my_stars, attempted) = match user {
        Some(user) => (
            Puzzle::stars_of(puzzle.id, user, conn).await?,
            Puzzle::attempted(puzzle.id, user, conn).await?,
        ),
        None => (None, false),
    };
    Ok(PuzzleResponse::from_model(puzzle, my_stars, attempted))
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
        length,
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
pub async fn set_puzzle_stars(
    nanoid: String,
    stars: i16,
) -> Result<PuzzleStarsResponse, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::Puzzle};
    let user = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let puzzle = Puzzle::find_by_nanoid(&nanoid, &mut conn).await?;
    let updated = Puzzle::set_stars(puzzle.id, user, stars, &mut conn).await?;
    Ok(PuzzleStarsResponse {
        my_stars: stars,
        average_stars: updated.average_stars(),
        stars_count: updated.stars_count,
    })
}

#[server(client = CsrfClient)]
pub async fn puzzle_history(
    filter: PuzzleHistoryFilter,
    page: i64,
) -> Result<Vec<PuzzleHistoryRow>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::Puzzle};
    let user = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let rows = Puzzle::history(user, filter, page, &mut conn).await?;
    Ok(rows
        .into_iter()
        .map(|(attempt, puzzle, my_stars)| PuzzleHistoryRow {
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
            my_stars,
            themes: puzzle.themes(),
            title: puzzle.title.clone(),
            history: puzzle.history.clone(),
            position: puzzle.position.clone(),
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
    Ok(Some(PuzzleRatingResponse {
        rating: rating.rating.round() as i32,
        played: rating.played,
        won: rating.won,
        provisional: Certainty::from_deviation(rating.deviation) != Certainty::Rankable,
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
