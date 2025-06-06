use crate::game_result::GameResult;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameError {
    #[error("Not a valid game move: {reason} turn: {turn}, piece: {piece}, current_position: {from}, target_position: {to}.")]
    InvalidMove {
        piece: String,
        from: String,
        to: String,
        turn: usize,
        reason: String,
    },
    #[error("Found {found:?} which is not a valid {typ}")]
    ParsingError { found: String, typ: String },
    #[error("Result {reported_result:?} doesn't match board endstate {actual_result:?}")]
    ResultMismatch {
        reported_result: GameResult,
        actual_result: GameResult,
    },
    #[error("No .pgn file supplied")]
    NoPgnFile,
    #[error("Invalid direction {direction:?}")]
    InvalidDirection { direction: String },
    #[error("Invalid color choice {found:?}")]
    InvalidColorChoice { found: String },
    #[error("{username} can't play on {game} at {turn}")]
    InvalidTurn {
        username: String,
        game: String,
        turn: String,
    },
    #[error("{gc} can't be played on {game} at {turn}")]
    InvalidGc {
        gc: String,
        game: String,
        turn: String,
    },
    #[error("{gc} already newest game gontrol on {game} at {turn}")]
    GcAlreadyPresent {
        gc: String,
        game: String,
        turn: String,
    },
    #[error("{username} can't play on {game}. Game is over.")]
    GameIsOver { username: String, game: String },
    #[error("{username} can't play on {game}. It's not their game.")]
    NotPlayer { username: String, game: String },
    #[error("Cannot abort tournament game")]
    TournamentAbort,
}

impl GameError {
    pub fn update_reason<S>(&mut self, reason_new: S)
    where
        S: Into<String>,
    {
        if let GameError::InvalidMove {
            piece: _,
            from: _,
            to: _,
            turn: _,
            ref mut reason,
        } = self
        {
            *reason = reason_new.into();
        }
    }

    pub fn update_from<S>(&mut self, from_new: S)
    where
        S: Into<String>,
    {
        if let GameError::InvalidMove {
            piece: _,
            from,
            to: _,
            turn: _,
            reason: _,
        } = self
        {
            *from = from_new.into();
        }
    }
}

impl From<anyhow::Error> for GameError {
    fn from(error: anyhow::Error) -> Self {
        GameError::ParsingError {
            found: error.to_string(),
            typ: "anyhow error".to_string(),
        }
    }
}
