use hive_lib::{GameControl, Turn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameReaction {
    Adjudicated,
    Berserk,
    Reopened,
    Started,
    Control(GameControl),
    Join,
    Turn(Turn),
    Ready,
    New,
    TimedOut,
    Finished,
    Tv,
}

impl fmt::Display for GameReaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameReaction::Adjudicated => write!(f, "Adjudicated"),
            GameReaction::Berserk => write!(f, "Berserk"),
            GameReaction::Reopened => write!(f, "Reopened"),
            GameReaction::Control(ref gc) => write!(f, "{gc}"),
            GameReaction::Join => write!(f, "Join"),
            GameReaction::Started => write!(f, "Started"),
            GameReaction::Turn(ref turn) => write!(f, "{turn}"),
            GameReaction::New => write!(f, "New"),
            GameReaction::Ready => write!(f, "Ready"),
            GameReaction::TimedOut => write!(f, "TimedOut"),
            GameReaction::Finished => write!(f, "Finished"),
            GameReaction::Tv => write!(f, "Tv"),
        }
    }
}
