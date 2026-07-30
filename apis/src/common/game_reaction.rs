use hive_lib::{GameControl, Turn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameReaction {
    Started,
    Control(GameControl),
    Join,
    Turn(Turn),
    Ready,
    New,
    TimedOut,
    Tv,
    /// A player declared berserk before the game started. Both sides need it:
    /// it halves that player's clock and drops their increment, so the other
    /// side's timer display changes too.
    Berserk,
}

impl fmt::Display for GameReaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameReaction::Control(ref gc) => write!(f, "{gc}"),
            GameReaction::Join => write!(f, "Join"),
            GameReaction::Started => write!(f, "Started"),
            GameReaction::Turn(ref turn) => write!(f, "{turn}"),
            GameReaction::New => write!(f, "New"),
            GameReaction::Ready => write!(f, "Ready"),
            GameReaction::TimedOut => write!(f, "TimedOut"),
            GameReaction::Tv => write!(f, "Tv"),
            GameReaction::Berserk => write!(f, "Berserk"),
        }
    }
}
