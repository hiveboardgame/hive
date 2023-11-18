use crate::position::Position;
use crate::piece::Piece;
use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Turn {
    Move(Piece, Position),
    Shutout,
    Spawn(Piece, Position),
}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Turn::Move(piece, pos) => format!("Moving({},{})", piece, pos),
            Turn::Shutout => String::from("Shutout"),
            Turn::Spawn(piece, pos) => format!("Spawning({},{})", piece, pos),
        };
        write!(f, "{}", name)
    }
}
