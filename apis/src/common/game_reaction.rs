use hive_lib::{game_control::GameControl, turn::Turn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameReaction {
    Control(GameControl),
    Join,
    Turn(Turn),
    New,
    TimedOut,
    Tv,
}

impl fmt::Display for GameReaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameReaction::Control(ref gc) => write!(f, "{}", gc),
            GameReaction::Join => write!(f, "Join"),
            GameReaction::Turn(ref turn) => write!(f, "{}", turn),
            GameReaction::New => write!(f, "New"),
            GameReaction::TimedOut => write!(f, "TimedOut"),
            GameReaction::Tv => write!(f, "Tv"),
        }
    }
}
