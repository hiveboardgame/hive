use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PuzzleResponse {
    pub nanoid: String,
    pub game_type: String,
    pub history: Option<String>,
    pub position: Option<String>,
    pub lines: Vec<Vec<String>>,
    pub solver_moves: i32,
    pub themes: Vec<String>,
    pub authors: Vec<String>,
    pub source: String,
    pub source_game: Option<String>,
    pub collection: Option<String>,
    pub title: Option<String>,
    pub hint: Option<String>,
    pub level: Option<String>,
    pub rating: i32,
    pub plays: i32,
    pub solves: i32,
    pub likes: i32,
    pub dislikes: i32,
    /// The viewer's own 👍 (true) or 👎 (false), when signed in and given.
    pub my_vote: Option<bool>,
    /// Whether the viewer already tried it; a repeat is an unrated replay.
    pub attempted: bool,
    /// The day it was the puzzle of the day, if ever.
    pub daily_day: Option<chrono::NaiveDate>,
}

impl PuzzleResponse {
    #[cfg(feature = "ssr")]
    pub fn from_model(
        puzzle: &db_lib::models::Puzzle,
        my_vote: Option<bool>,
        attempted: bool,
        daily_day: Option<chrono::NaiveDate>,
    ) -> Self {
        Self {
            nanoid: puzzle.nanoid.clone(),
            game_type: puzzle.game_type.clone(),
            history: puzzle.history.clone(),
            position: puzzle.position.clone(),
            lines: puzzle.lines(),
            solver_moves: puzzle.solver_moves,
            themes: puzzle.themes(),
            authors: puzzle.authors(),
            source: puzzle.source.clone(),
            source_game: puzzle.source_game.clone(),
            collection: puzzle.collection.clone(),
            title: puzzle.title.clone(),
            hint: puzzle.hint.clone(),
            level: puzzle.level.clone(),
            rating: puzzle.rating.round() as i32,
            plays: puzzle.plays,
            solves: puzzle.solves,
            likes: puzzle.likes,
            dislikes: puzzle.dislikes,
            my_vote,
            attempted,
            daily_day,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PuzzleAttemptResponse {
    pub rated: bool,
    /// Whether this try was added to the puzzle's play count (a first try).
    pub counted_play: bool,
    /// Rounded once, from the exact ratings, so every page shows the same change.
    pub rating_change: Option<i32>,
    pub rating_before: Option<i32>,
    pub rating_after: Option<i32>,
    pub puzzle_rating_after: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PuzzleVoteResponse {
    pub my_vote: Option<bool>,
    pub likes: i32,
    pub dislikes: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PuzzleHistoryRow {
    pub nanoid: String,
    pub game_type: String,
    pub solver_moves: i32,
    pub rating: i32,
    pub solved: bool,
    pub hinted: bool,
    pub rating_change: Option<i32>,
    pub played_at: DateTime<Utc>,
    pub my_vote: Option<bool>,
    pub themes: Vec<String>,
    pub title: Option<String>,
    pub history: Option<String>,
    pub position: Option<String>,
    pub daily_day: Option<chrono::NaiveDate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PuzzleRatingResponse {
    pub rating: i32,
    pub played: i64,
    pub won: i64,
    pub provisional: bool,
    pub streak: PuzzleStreakResponse,
}

/// One browsable set: a theme, `moves:N` or `collection:X`, with the viewer's clean solves.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PuzzleSetResponse {
    pub key: String,
    pub total: i64,
    pub solved: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DailyPuzzleResponse {
    pub day: chrono::NaiveDate,
    pub puzzle: PuzzleResponse,
    pub tried: i64,
    pub solved: i64,
    /// The viewer's first try: solved without help, or missed.
    pub my_result: Option<bool>,
    pub streak: Option<PuzzleStreakResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DailyQueueEntry {
    pub nanoid: String,
    pub day: chrono::NaiveDate,
    pub game_type: String,
    pub solver_moves: i32,
    pub rating: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DailyScheduleResponse {
    pub recent: Vec<(chrono::NaiveDate, String)>,
    pub queued: Vec<DailyQueueEntry>,
}

/// Days in a row the viewer solved the puzzle of the day on its day.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PuzzleStreakResponse {
    pub current: i64,
    pub best: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DailyArchiveRow {
    pub day: chrono::NaiveDate,
    pub nanoid: String,
    pub game_type: String,
    pub history: Option<String>,
    pub position: Option<String>,
    pub solver_moves: i32,
    pub rating: i32,
    pub my_result: Option<bool>,
}
