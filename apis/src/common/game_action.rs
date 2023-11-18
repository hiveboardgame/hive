use hive_lib::{game_control::GameControl, turn::Turn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAction {
    Move(Turn),
    Control(GameControl),
    Chat(String),
    Join(String),
}
