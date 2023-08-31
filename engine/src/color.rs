use crate::game_error::GameError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug, Default)]
#[repr(u8)]
pub enum Color {
    #[default]
    White = 0,
    Black = 1,
}

impl FromStr for Color {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "w" => Ok(Color::White),
            "b" => Ok(Color::Black),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "color string".to_string(),
            }),
        }
    }
}

impl From<u8> for Color {
    fn from(num: u8) -> Self {
        if num == 0 {
            return Color::White;
        }
        Color::Black
    }
}

impl From<Color> for u8 {
    fn from(color: Color) -> Self {
        color as u8
    }
}

impl Color {
    pub fn opposite(&self) -> u8 {
        1 - (*self as u8)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::White => "white",
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let color = match self {
            Color::White => "w",
            Color::Black => "b",
        };
        write!(f, "{color}")
    }
}
