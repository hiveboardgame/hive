use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Direction {
    NW,
    NE,
    E,
    SE,
    SW,
    W,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Direction::NW => write!(f, "NorthWest"),
            Direction::NE => write!(f, "NorthEast"),
            Direction::E => write!(f, "East"),
            Direction::SE => write!(f, "SouthEast"),
            Direction::SW => write!(f, "SouthWest"),
            Direction::W => write!(f, "West"),
        }
    }
}

impl Direction {
    pub fn all() -> Vec<Direction> {
        vec![
            Direction::NW,
            Direction::NE,
            Direction::E,
            Direction::SE,
            Direction::SW,
            Direction::W,
        ]
    }

    pub fn adjacent_directions(&self) -> (Direction, Direction) {
        match self {
            Direction::NW => (Direction::W, Direction::NE),
            Direction::NE => (Direction::NW, Direction::E),
            Direction::E => (Direction::NE, Direction::SE),
            Direction::SE => (Direction::E, Direction::SW),
            Direction::SW => (Direction::SE, Direction::W),
            Direction::W => (Direction::SW, Direction::NW),
        }
    }

    pub fn to_history_string(&self, piece: String) -> String {
        let piece = piece.replace(' ', "");
        match self {
            Direction::NE => piece + "/",
            Direction::E => piece + "-",
            Direction::SE => piece + "\\",
            Direction::NW => "\\".to_string() + &piece,
            Direction::SW => "/".to_string() + &piece,
            Direction::W => "-".to_string() + &piece,
        }
    }
}
