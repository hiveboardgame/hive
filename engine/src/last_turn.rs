use crate::position::Position;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum LastTurn {
    Pass,
    Shutout,
    Move(Position, Position),
    #[default]
    None,
}
