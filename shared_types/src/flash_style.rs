use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlashStyle {
    Timer,
    #[default]
    Board,
    Screen,
}

impl FlashStyle {
    pub const ALL: [FlashStyle; 3] = [FlashStyle::Timer, FlashStyle::Board, FlashStyle::Screen];
}

impl fmt::Display for FlashStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            FlashStyle::Timer => "Timer",
            FlashStyle::Board => "Board",
            FlashStyle::Screen => "Screen",
        };
        write!(f, "{name}")
    }
}

impl FromStr for FlashStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Timer" => Ok(FlashStyle::Timer),
            "Board" => Ok(FlashStyle::Board),
            "Screen" => Ok(FlashStyle::Screen),
            _ => Err(()),
        }
    }
}
