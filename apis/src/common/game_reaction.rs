use hive_lib::{GameControl, Turn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameReaction {
    Started,
    Control(GameControl),
    Turn(Turn),
    Ready,
    New,
    TimedOut,
    Tv,
}

impl fmt::Display for GameReaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameReaction::Control(ref gc) => write!(f, "{}", gc),
            GameReaction::Started => write!(f, "Started"),
            GameReaction::Turn(ref turn) => write!(f, "{}", turn),
            GameReaction::New => write!(f, "New"),
            GameReaction::Ready => write!(f, "Ready"),
            GameReaction::TimedOut => write!(f, "TimedOut"),
            GameReaction::Tv => write!(f, "Tv"),
        }
    }
}
