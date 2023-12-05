use hive_lib::{game_control::GameControl, turn::Turn};
use serde::{Deserialize, Serialize};
use std::fmt;
//use db_lib::{models::game::Game, DbPool};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAction {
    Chat(String),
    Control(GameControl),
    Join,
    Move(Turn),
}

impl fmt::Display for GameAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameAction::Chat(msg) => write!(f, "Message is: {msg}"),
            GameAction::Control(ref gc) => write!(f, "{}", gc),
            GameAction::Join => write!(f, "Join"),
            GameAction::Move(ref turn) => write!(f, "{}", turn),
        }
    }
}
