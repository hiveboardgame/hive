use crate::game_result::GameResult;

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
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
