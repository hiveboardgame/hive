use hive_lib::{Piece, Position};

use crate::common::PieceType;

#[derive(Debug, Clone, PartialEq)]
pub struct MoveInfo {
    // the piece (either from reserve or board) that has been clicked last
    pub active: Option<(Piece, PieceType)>,
    // the position of the board piece that has been clicked last
    pub current_position: Option<Position>,
    // possible destinations of selected piece
    pub target_positions: Vec<Position>,
    // premove targets that are only reachable if the opponent's piece there moves away
    pub vacate_targets: Vec<Position>,
    // the position of the target that got clicked last
    pub target_position: Option<Position>,
    // the position of the reserve piece that got clicked last
    pub reserve_position: Option<Position>,
    // active -> target_position is queued to play once the opponent has moved
    pub premove: bool,
}

impl Default for MoveInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveInfo {
    pub fn new() -> Self {
        Self {
            active: None,
            current_position: None,
            target_positions: vec![],
            vacate_targets: vec![],
            target_position: None,
            reserve_position: None,
            premove: false,
        }
    }

    pub fn reset(&mut self) {
        self.target_positions.clear();
        self.vacate_targets.clear();
        self.active = None;
        self.target_position = None;
        self.current_position = None;
        self.reserve_position = None;
        self.premove = false;
    }

    pub fn queued_premove(&self) -> Option<(Piece, Position)> {
        if !self.premove {
            return None;
        }
        self.active
            .map(|(piece, _)| piece)
            .zip(self.target_position)
    }
}
