use crate::bug::Bug;
use crate::color::Color;
use crate::game_error::GameError;
use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[bitfield(u8)]
#[derive(Serialize, Deserialize, PartialEq, Hash, Eq, PartialOrd, Ord)]
pub struct Piece {
    #[bits(1)]
    pub color: Color,
    #[bits(3)]
    pub bug: Bug,
    #[bits(2)]
    pub order: usize,
    #[bits(1)]
    pub invalid: bool,
    /// we need to fill the u8
    #[bits(1)]
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
    pub fn simple(&self) -> u8 {
        u8::from(*self) & 0b00001111
    }

    pub fn to_u5(&self) -> u8 {
        let bug = match self.bug() {
            Bug::Ant => self.order() as u8,
            Bug::Beetle => 3 + self.order() as u8,
            Bug::Grasshopper => 5 + self.order() as u8,
            Bug::Spider => 8 + self.order() as u8,
            Bug::Mosquito => 11,
            Bug::Ladybug => 12,
            Bug::Pillbug => 13,
            Bug::Queen => 14,
        };
        let color = match self.color() {
            Color::White => 0,
            Color::Black => 14,
        };
        bug + color
    }

    pub fn from_u5(u5: u8) -> Piece {
        match u5 {
            1 => Piece::new()
                .with_bug(Bug::Ant)
                .with_color(Color::White)
                .with_order(1),
            2 => Piece::new()
                .with_bug(Bug::Ant)
                .with_color(Color::White)
                .with_order(2),
            3 => Piece::new()
                .with_bug(Bug::Ant)
                .with_color(Color::White)
                .with_order(3),
            4 => Piece::new()
                .with_bug(Bug::Beetle)
                .with_color(Color::White)
                .with_order(1),
            5 => Piece::new()
                .with_bug(Bug::Beetle)
                .with_color(Color::White)
                .with_order(2),
            6 => Piece::new()
                .with_bug(Bug::Grasshopper)
                .with_color(Color::White)
                .with_order(1),
            7 => Piece::new()
                .with_bug(Bug::Grasshopper)
                .with_color(Color::White)
                .with_order(2),
            8 => Piece::new()
                .with_bug(Bug::Grasshopper)
                .with_color(Color::White)
                .with_order(3),
            9 => Piece::new()
                .with_bug(Bug::Spider)
                .with_color(Color::White)
                .with_order(1),
            10 => Piece::new()
                .with_bug(Bug::Spider)
                .with_color(Color::White)
                .with_order(2),
            11 => Piece::new()
                .with_bug(Bug::Mosquito)
                .with_color(Color::White),
            12 => Piece::new().with_bug(Bug::Ladybug).with_color(Color::White),
            13 => Piece::new().with_bug(Bug::Pillbug).with_color(Color::White),
            14 => Piece::new().with_bug(Bug::Queen).with_color(Color::White),
            15 => Piece::new()
                .with_bug(Bug::Ant)
                .with_color(Color::Black)
                .with_order(1),
            16 => Piece::new()
                .with_bug(Bug::Ant)
                .with_color(Color::Black)
                .with_order(2),
            17 => Piece::new()
                .with_bug(Bug::Ant)
                .with_color(Color::Black)
                .with_order(3),
            18 => Piece::new()
                .with_bug(Bug::Beetle)
                .with_color(Color::Black)
                .with_order(1),
            19 => Piece::new()
                .with_bug(Bug::Beetle)
                .with_color(Color::Black)
                .with_order(2),
            20 => Piece::new()
                .with_bug(Bug::Grasshopper)
                .with_color(Color::Black)
                .with_order(1),
            21 => Piece::new()
                .with_bug(Bug::Grasshopper)
                .with_color(Color::Black)
                .with_order(2),
            22 => Piece::new()
                .with_bug(Bug::Grasshopper)
                .with_color(Color::Black)
                .with_order(3),
            23 => Piece::new()
                .with_bug(Bug::Spider)
                .with_color(Color::Black)
                .with_order(1),
            24 => Piece::new()
                .with_bug(Bug::Spider)
                .with_color(Color::Black)
                .with_order(2),
            25 => Piece::new()
                .with_bug(Bug::Mosquito)
                .with_color(Color::Black),
            26 => Piece::new().with_bug(Bug::Ladybug).with_color(Color::Black),
            27 => Piece::new().with_bug(Bug::Pillbug).with_color(Color::Black),
            28 => Piece::new().with_bug(Bug::Queen).with_color(Color::Black),
            any => panic!("Got an invalid u5 bug: {}", any),
        }
    }

    pub fn to_char(&self) -> char {
        char::from_u32(65_u32 + self.simple() as u32).unwrap()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_order() {
        let wq = Piece::new().with_bug(Bug::Queen).with_color(Color::White);
        let bq = Piece::new().with_bug(Bug::Queen).with_color(Color::Black);
        assert!(wq < bq);
        assert!(wq.simple() < bq.simple());
        let ba1 = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::Black)
            .with_order(1);
        let ba2 = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::Black)
            .with_order(2);
        assert!(ba1.simple() == ba2.simple());
        let wa1 = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::White)
            .with_order(1);
        let ba2 = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::Black)
            .with_order(2);
        assert!(wa1.simple() < ba2.simple());
        let wa1 = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::White)
            .with_order(1);
        let wa2 = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::White)
            .with_order(2);
        assert!(wa1.simple() <= wa2.simple());
        assert!(wa1.simple() >= wa2.simple());
    }

    #[test]
    fn tests_simple() {
        let piece = Piece::new()
            .with_bug(Bug::Ant)
            .with_color(Color::Black)
            .with_order(1);
        let simple_piece = Piece::new().with_bug(Bug::Ant).with_color(Color::Black);
        assert_eq!(piece.simple(), simple_piece.into());
    }

    #[test]
    fn test_u5() {
        for u5 in 1..29 {
            assert_eq!(Piece::from_u5(u5).to_u5(), u5);
        }
    }
}
