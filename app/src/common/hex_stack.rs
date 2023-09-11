use crate::common::{
    hex::{Direction, Hex, HexType},
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

    pub fn len(&self) -> usize {
        self.hexes.len()
    }

    pub fn add_active(&mut self, piece: Piece) {
        if self.hexes.iter().any(|hex| hex.kind == HexType::LastMove) {
            self.hexes.push(Hex {
                kind: HexType::Tile(piece, PieceType::Spawn),
                position: self.position,
                level: self.hexes.len() - 1,
            })
        } else {
            self.hexes.push(Hex {
                kind: HexType::Tile(piece, PieceType::Spawn),
                position: self.position,
                level: self.hexes.len(),
            })
        }
    }

    pub fn add_target(&mut self) {
        if self.hexes.iter().any(|hex| hex.kind == HexType::LastMove) {
            self.hexes.push(Hex {
                kind: HexType::Target,
                position: self.position,
                level: self.hexes.len() - 1,
            })
        } else {
            self.hexes.push(Hex {
                kind: HexType::Target,
                position: self.position,
                level: self.hexes.len(),
            })
        }
    }

    pub fn add_last_move(&mut self, direction: Direction) {
        match direction {
            Direction::To => {
                let top = self.hexes.pop();
                self.hexes.push(Hex {
                    kind: HexType::LastMove,
                    position: self.position,
                    level: self.hexes.len(),
                });
                if let Some(piece) = top {
                    self.hexes.push(piece)
                }
            }
            Direction::From => self.hexes.push(Hex {
                kind: HexType::LastMove,
                position: self.position,
                level: self.hexes.len(),
            }),
        }
    }
}
