use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::{board::Board, history::History};

#[derive(Serialize, Clone, Default, Deserialize, Debug, PartialEq, Eq)]
pub struct Hasher {
    pub moves: Vec<Vec<u8>>,
    pub states: Vec<Vec<u8>>,
}

impl Hasher {
    pub fn new() -> Self {
        Self {
            moves: Vec::new(),
            states: Vec::new(),
        }
    }

    pub fn record_board_state(&mut self, board: &Board) {
        let s = format!("{board}");
        let mut hasher = Sha512::new();
        hasher.update(s);
        self.moves.push(hasher.finalize().to_vec());
    }

    pub fn record_move(&mut self, history: &History) {
        let mut hasher = Sha512::new();
        let mut s = String::new();
        for (piece, pos) in history.moves.iter() {
            s.push_str(piece);
            s.push_str(pos);
        }
        hasher.update(s);
        self.states.push(hasher.finalize().to_vec());
    }
}
