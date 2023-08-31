use crate::color::Color;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Player {
    color: Color,
}

impl Player {
    pub fn new(color: Color) -> Player {
        Player { color }
    }
}
