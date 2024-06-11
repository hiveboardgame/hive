use hive_lib::{Piece, Position};

#[derive(Debug, Clone, PartialEq)]
pub struct MoveInfo {
    // the piece (either from reserve or board) that has been clicked last
    pub active: Option<Piece>,
    // the position of the board piece that has been clicked last
    pub current_position: Option<Position>,
    // possible destinations of selected piece
    pub target_positions: Vec<Position>,
    // the position of the target that got clicked last
    pub target_position: Option<Position>,
    // the position of the reserve piece that got clicked last
    pub reserve_position: Option<Position>,
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
            target_position: None,
            reserve_position: None,
        }
    }

    pub fn reset(&mut self) {
        self.target_positions.clear();
        self.active = None;
        self.target_position = None;
        self.current_position = None;
        self.reserve_position = None;
    }
}
