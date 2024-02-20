use crate::color::Color;
use crate::piece::Piece;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugStack {
    pub pieces: [Piece; 7],
    pub size: u8,
}

impl Default for BugStack {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BugStack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BugStack len: {} top_piece: {:?} pieces: {:?}",
            self.len(),
            self.top_piece(),
            self.pieces
        )
    }
}

impl BugStack {
    pub fn new() -> Self {
        Self {
            pieces: [Piece::new(); 7],
            size: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.size as usize
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn top_bug_color(&self) -> Option<Color> {
        if self.is_empty() {
            return None;
        }
        Some(self.pieces[self.size as usize].color())
    }

    pub fn push_piece(&mut self, piece: Piece) {
        if self.size == 7 {
            panic!("Trying to add an 8th bug to a BugStack")
        }
        self.pieces[self.size as usize] = piece;
        self.size += 1;
    }

    pub fn pop_piece(&mut self) -> Piece {
        if self.size == 0 {
            panic!("Trying to remove a bug from an empty BugStack")
        }
        self.size -= 1;
        let piece = self.pieces[self.size as usize];
        self.pieces[self.size as usize] = Piece::new();
        piece
    }

    pub fn top_piece(&self) -> Option<Piece> {
        if self.size == 0 {
            return None;
        }
        Some(self.pieces[(self.size - 1) as usize])
    }

    pub fn under_piece(&self) -> Option<Piece> {
        if self.size <= 1 {
            return None;
        }
        Some(self.pieces[(self.size - 2) as usize])
    }

    pub fn bottom_piece(&self) -> Option<Piece> {
        if self.size == 0 {
            return None;
        }
        Some(self.pieces[0])
    }
}
