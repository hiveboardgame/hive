use crate::piece::Piece;
use crate::position::Position;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Turn {
    Move(Piece, Position),
    Shutout,
}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Turn::Move(piece, pos) => format!("Moving({piece},{pos})"),
            Turn::Shutout => String::from("Shutout"),
        };
        write!(f, "{name}")
    }
}
