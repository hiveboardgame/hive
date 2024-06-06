use crate::{position::Rotation, GameError};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"([-/\\]?)([wb][ABGMLPSQ]\d?)([-/\\]?)").expect("This regex should compile");
}

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
    pub fn to_u8(&self) -> u8 {
        use Direction::*;
        match self {
            NE => 1,
            E => 2,
            SE => 3,
            NW => 4,
            SW => 5,
            W => 6,
        }
    }

    pub fn from_u8(dir: u8) -> Result<Direction, GameError> {
        use Direction::*;
        match dir {
            1 => Ok(NE),
            2 => Ok(E),
            3 => Ok(SE),
            4 => Ok(NW),
            5 => Ok(SW),
            6 => Ok(W),
            other => Err(GameError::ParsingError {
                found: format!("Found {} for direction", other),
                typ: String::from("Direction"),
            }),
        }
    }

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

    pub fn from_string(s: &str) -> Option<Direction> {
        if s.starts_with('.') || s.is_empty() {
            return None;
        }
        if let Some(cap) = RE.captures(s) {
            if !cap[1].is_empty() {
                return match &cap[1] {
                    "\\" => Some(Direction::NW),
                    "-" => Some(Direction::W),
                    "/" => Some(Direction::SW),
                    _ => None,
                };
            }
            if !cap[3].is_empty() {
                return match &cap[3] {
                    "/" => Some(Direction::NE),
                    "-" => Some(Direction::E),
                    "\\" => Some(Direction::SE),
                    _ => None,
                };
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order() {}
}
