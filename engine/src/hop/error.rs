use thiserror::Error;

use crate::{bug::Bug, color::Color, game_type::GameType};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum HopError {
    #[error("HOP string is empty")]
    Empty,
    #[error("HOP must have 2 or 3 comma-separated fields, found {0}")]
    FieldCount(usize),
    #[error("unsupported game type: {0:?}")]
    UnsupportedGameType(String),
    #[error("the Dragonfly is not supported")]
    Dragonfly,
    #[error("unexpected character in topology: {0:?}")]
    BadChar(char),
    #[error("topology contains no bugs")]
    NoStartBug,
    #[error("chain position {0} referenced before it exists")]
    BadChainRef(usize),
    #[error("expected a bug after '='")]
    MissingStackBug,
    #[error("expected '=', '+' or '-' after chain position {0}")]
    BadChainOp(usize),
    #[error("unbalanced parentheses in topology")]
    UnbalancedParens,
    #[error("invalid player-to-move: {0:?}")]
    BadPlayer(String),
    #[error("{bug:?} is not part of game type {game_type}")]
    PieceNotInGameType { bug: Bug, game_type: GameType },
    #[error("too many {color} {bug:?} pieces for this game type")]
    TooManyPieces { color: Color, bug: Bug },
}
