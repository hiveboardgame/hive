use serde::{Deserialize, Serialize};
use std::fmt;

use crate::position::Rotation;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Direction {
    NW = 1,
    NE,
    E,
    SE,
    SW,
    W,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Direction::*;
        match self {
            NW => write!(f, "NorthWest"),
            NE => write!(f, "NorthEast"),
            E => write!(f, "East"),
            SE => write!(f, "SouthEast"),
            SW => write!(f, "SouthWest"),
            W => write!(f, "West"),
        }
    }
}

impl Direction {
    pub fn next_direction(&self, revolution: Rotation) -> Direction {
        use Direction::*;
        match revolution {
            Rotation::C => match self {
                E => SE,
                SE => SW,
                SW => W,
                W => NW,
                NW => NE,
                NE => E,
            },
            Rotation::CC => match self {
                W => SW,
                SW => SE,
                SE => E,
                E => NE,
                NE => NW,
                NW => W,
            },
        }
    }

    pub fn next_direction_120(&self) -> Direction {
        use Direction::*;
        match self {
            NE => SE,
            E => SW,
            SE => W,
            SW => NW,
            W => NE,
            NW => E,
        }
    }

    pub fn all() -> Vec<Direction> {
        use Direction::*;
        vec![NW, NE, E, SE, SW, W]
    }

    pub fn adjacent_directions(&self) -> (Direction, Direction) {
        use Direction::*;
        match self {
            NW => (W, NE),
            NE => (NW, E),
            E => (NE, SE),
            SE => (E, SW),
            SW => (SE, W),
            W => (SW, NW),
        }
    }

    pub fn to_history_string(&self, piece: String) -> String {
        let piece = piece.replace(' ', "");
        use Direction::*;
        match self {
            NE => piece + "/",
            E => piece + "-",
            SE => piece + "\\",
            NW => "\\".to_string() + &piece,
            SW => "/".to_string() + &piece,
            W => "-".to_string() + &piece,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order() {
        assert_eq!(Direction::(1), Direction::NW);
    }
}
