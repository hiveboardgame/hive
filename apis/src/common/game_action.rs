use hive_lib::{GameControl, Turn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAction {
    Start,
    Berserk,
    // Sent by tabs from before ClockRanOut, without the turn whose clock ran out.
    CheckTime,
    ClockRanOut(usize),
    Control(GameControl),
    Join,
    Turn(Turn),
    Play(String),
    PlayAt { turn: usize, notation: String },
    Unwatch,
}

impl fmt::Display for GameAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameAction::CheckTime => write!(f, "CheckTime"),
            GameAction::ClockRanOut(turn) => write!(f, "ClockRanOut({turn})"),
            GameAction::Berserk => write!(f, "Berserk"),
            GameAction::Control(ref gc) => write!(f, "{gc}"),
            GameAction::Join => write!(f, "Join"),
            GameAction::Start => write!(f, "Start"),
            GameAction::Turn(ref turn) => write!(f, "{turn}"),
            GameAction::Play(ref notation) => write!(f, "{notation}"),
            GameAction::PlayAt { ref notation, .. } => write!(f, "{notation}"),
            GameAction::Unwatch => write!(f, "Unwatch"),
        }
    }
}
