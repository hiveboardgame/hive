use crate::{piece::Piece, position::Position};
use std::fmt;

#[derive(Clone, Debug)]
pub struct DfsInfo {
    pub position: Position,
    pub parent: Option<usize>,
    pub piece: Piece,
    pub visited: bool,
    pub depth: usize,
    pub low: usize,
    pub pinned: bool,
}

impl fmt::Display for DfsInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {} {}", self.piece, self.pinned, self.visited)
    }
}
