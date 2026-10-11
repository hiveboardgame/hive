use crate::game::Action;
use anyhow::{anyhow, Result};
use hive_lib::{GameStatus, State};

/// The site's `piece position` notation for `action` played from `state`.
pub fn notation(state: &State, action: &Action) -> String {
    match action {
        Action::Move { piece, to, .. } | Action::Place { piece, to } => {
            let (piece, position) = state.move_notation(*piece, *to);
            if position.is_empty() {
                piece
            } else {
                format!("{piece} {position}")
            }
        }
        Action::Pass => "pass".to_string(),
    }
}

/// Plays `action` the way the site does (auto-passing a shut-out opponent). A `Pass` the state
/// already auto-played is consumed silently, so a line with explicit passes replays as-is.
pub fn play(state: &mut State, action: &Action) -> Result<()> {
    match action {
        Action::Pass => {
            if state.history.last_move_is_pass() {
                return Ok(());
            }
            state.play_turn_from_history("pass", "")?;
        }
        Action::Move { piece, to, .. } | Action::Place { piece, to } => {
            if matches!(
                state.game_status,
                GameStatus::Finished(_) | GameStatus::Adjudicated
            ) {
                return Err(anyhow!("game already over before {piece}"));
            }
            state.play_turn_from_position(*piece, *to)?;
        }
    }
    Ok(())
}
