use crate::common::{
    hex::{ActiveState, Direction, Hex, HexType},
    piece_type::PieceType,
};
use hive_lib::{bug_stack::BugStack, piece::Piece, position::Position};

#[derive(Debug)]
pub struct HexStack {
    pub position: Position,
    pub hexes: Vec<Hex>,
}

impl HexStack {
    pub fn new(bug_stack: &BugStack, position: Position) -> Self {
        let last = bug_stack.len();
        HexStack {
            position,
            hexes: (0..bug_stack.len())
                .map(|i| {
                    if i + 1 == last {
                        Hex {
                            kind: HexType::Tile(bug_stack.pieces[i], PieceType::Board),
                            position,
                            level: i,
                        }
                    } else {
                        Hex {
                            kind: HexType::Tile(bug_stack.pieces[i], PieceType::Covered),
                            position,
                            level: i,
                        }
                    }
                })
                .collect(),
        }
    }

    pub fn new_history(bug_stack: &BugStack, position: Position) -> Self {
        HexStack {
            position,
            hexes: (0..bug_stack.len())
                .map(|i| Hex {
                    kind: HexType::Tile(bug_stack.pieces[i], PieceType::History),
                    position,
                    level: i,
                })
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.hexes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hexes.len() == 0
    }

    pub fn add_active(&mut self, target_selected: bool) {
        let mut len = self.len();
        if self.hexes.iter().any(|hex| {
            hex.kind == HexType::LastMove(Direction::From)
                || hex.kind == HexType::LastMove(Direction::To)
        }) {
            len -= 1
        }
        if target_selected {
            self.hexes.pop();
        }
        self.hexes.push(Hex {
            kind: HexType::Active(self.active_state()),
            position: self.position,
            level: len.saturating_sub(1),
        });
    }

    pub fn active_state(&self) -> ActiveState {
        if let Some(hex) = self.hexes.last() {
            return match &hex.kind {
                HexType::Target => ActiveState::Board,
                HexType::Tile(_, piece_type) => match piece_type {
                    PieceType::Board | PieceType::Spawn => ActiveState::Board,
                    PieceType::Reserve => ActiveState::Reserve,
                    _ => ActiveState::None,
                },
                _ => ActiveState::None,
            };
        };
        ActiveState::None
    }

    pub fn add_tile(&mut self, piece: Piece, piece_type: PieceType) {
        let mut len = self.len();
        if self.hexes.iter().any(|hex| {
            hex.kind == HexType::LastMove(Direction::From)
                || hex.kind == HexType::LastMove(Direction::To)
        }) {
            len -= 1;
        }
        self.hexes.push(Hex {
            kind: HexType::Tile(piece, piece_type),
            position: self.position,
            level: len,
        })
    }

    pub fn add_target(&mut self) {
        let mut len = self.len();
        if self.hexes.iter().any(|hex| {
            hex.kind == HexType::LastMove(Direction::From)
                || hex.kind == HexType::LastMove(Direction::To)
        }) {
            len -= 1
        }
        self.hexes.push(Hex {
            kind: HexType::Target,
            position: self.position,
            level: len,
        })
    }

    pub fn add_last_move(&mut self, direction: Direction) {
        match direction {
            Direction::To => {
                let top = self.hexes.pop();
                self.hexes.push(Hex {
                    kind: HexType::LastMove(Direction::To),
                    position: self.position,
                    level: self.hexes.len(),
                });
                if let Some(piece) = top {
                    self.hexes.push(piece)
                }
            }
            Direction::From => self.hexes.push(Hex {
                kind: HexType::LastMove(Direction::From),
                position: self.position,
                level: self.hexes.len(),
            }),
        }
    }
}
