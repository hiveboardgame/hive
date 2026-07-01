use crate::GameSpeed;
use hudsoni::GameType;
use serde::{Deserialize, Serialize};

/// Default minimum game length (in plies/turns) for the opening explorer. Games shorter than
/// this are usually early resigns/timeouts that would skew opening statistics.
pub const MIN_PLIES: i32 = 8;

/// Filters applied when aggregating games for the opening explorer. All fields map directly to
/// columns denormalized onto every `game_hashes` row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExplorerFilters {
    /// Variant the explorer is browsing. Locked to one variant so suggested moves are
    /// achievable in the current game type.
    pub game_type: GameType,
    /// Speeds to include. Empty => no speed filter (all speeds).
    pub speeds: Vec<GameSpeed>,
    /// `Some(true)` => rated only (the default), `Some(false)` => casual only, `None` => both.
    pub rated: Option<bool>,
    /// Exclude games shorter than this many plies. Defaults to [`MIN_PLIES`].
    pub min_game_length: Option<i32>,
}

impl ExplorerFilters {
    pub fn new(game_type: GameType) -> Self {
        Self {
            game_type,
            speeds: GameSpeed::all_rated_games(),
            rated: Some(true),
            min_game_length: Some(MIN_PLIES),
        }
    }
}

/// One aggregated suggested move (or the current position itself, as a header). Keyed by the
/// resulting position hash; `piece`/`position` is a human-readable label only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExplorerMove {
    /// Canonical hash of the resulting position — the stable "link forward" to the next ply.
    pub next_hash: i64,
    pub piece: String,
    pub position: String,
    /// Number of distinct games that reached this continuation (counts games, not occurrences).
    pub total: i64,
    pub white_wins: i64,
    pub black_wins: i64,
    pub draws: i64,
    pub avg_rating: Option<f64>,
}
