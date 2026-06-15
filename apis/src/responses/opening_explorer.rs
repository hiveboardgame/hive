use crate::responses::GameResponse;
use serde::{Deserialize, Serialize};
use shared_types::ExplorerMove;

/// Opening-explorer payload for a single position: aggregate stats for the position itself, the
/// best continuations played from it, and the strongest games that passed through it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExplorerResponse {
    /// Aggregate stats for the queried position (the header). For the empty board this is the
    /// summed stats over all opening roots.
    pub position_total: ExplorerMove,
    /// Suggested next moves, ranked by popularity, keyed by their resulting position hash.
    pub moves: Vec<ExplorerMove>,
    /// Strongest games (by rating) that reached the queried position. Empty for the start.
    pub top_games: Vec<GameResponse>,
    /// Most recently played games that reached the queried position. Empty for the start.
    pub recent_games: Vec<GameResponse>,
}
