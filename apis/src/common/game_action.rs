use hive_lib::{GameControl, Turn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAction {
    Start,
    CheckTime,
    Control(GameControl),
    Join,
    Turn(Turn),
}

impl fmt::Display for GameAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameAction::CheckTime => write!(f, "CheckTime"),
            GameAction::Control(ref gc) => write!(f, "{}", gc),
            GameAction::Join => write!(f, "Join"),
            GameAction::Start => write!(f, "Start"),
            GameAction::Turn(ref turn) => write!(f, "{}", turn),
        }
    }
}
