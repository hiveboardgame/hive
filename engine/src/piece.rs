use crate::bug::Bug;
use crate::color::Color;
use crate::game_error::GameError;
use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[bitfield(u8)]
#[derive(Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct Piece {
    #[bits(1)]
    pub color: Color,
    #[bits(3)]
    pub bug: Bug,
    #[bits(2)]
    pub order: usize,
    /// we need to fill the u8
    #[bits(2)]
    _padding: usize,
}

impl FromStr for Piece {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(c_chars) = s.chars().next() {
            let color: Color = c_chars.to_string().parse()?;
            if let Some(b_chars) = s.chars().nth(1) {
                let bug: Bug = b_chars.to_string().parse()?;
                let mut order = 0;
                if let Some(ch) = s.chars().nth(2) {
                    if let Ok(ord) = ch.to_string().parse() {
                        order = ord;
                    }
                }
                match bug {
                    Bug::Ant | Bug::Beetle | Bug::Grasshopper | Bug::Spider if order == 0 => {
                        return Err(GameError::ParsingError {
                            found: s.to_string(),
                            typ: "piece".to_string(),
                        })
                    }
                    _ => {}
                }
                return Ok(Piece::new_from(bug, color, order));
            }
        }
        Err(GameError::ParsingError {
            found: s.to_string(),
            typ: "piece".to_string(),
        })
    }
}

impl Piece {
    pub fn new_from(bug: Bug, color: Color, order: usize) -> Piece {
        if bug.has_order() {
            return Piece::new()
                .with_color(color)
                .with_bug(bug)
                .with_order(order);
        }
        Piece::new().with_color(color).with_bug(bug)
    }

    pub fn is_color(&self, color: Color) -> bool {
        color == self.color()
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.order() > 0 {
            write!(f, "{}{}{}", self.color(), self.bug(), self.order())
        } else {
            write!(f, "{}{}", self.color(), self.bug())
        }
    }
}
