use crate::common::{
    hex::{Direction, Hex, HexType},
    piece_type::PieceType,
};
use hive_lib::{bug_stack::BugStack, position::Position};

#[derive(Debug)]
pub struct HexStack {
    pub position: Position,
    pub hexes: Vec<Hex>,
}

impl HexStack {
    pub fn new_from_target(position: Position) -> Self {
        Self {
            position,
            hexes: vec![Hex {
                kind: HexType::Target,
                position: position,
                level: 0,
            }],
        }
    }

    pub fn new_from_bugstack(bug_stack: &BugStack, position: Position) -> Self {
        let last = bug_stack.len() - 1;
        HexStack {
            position,
            hexes: (0..bug_stack.len())
                .map(|i| {
                    if i == last {
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

    pub fn new_from_last_move(position: Position) -> Self {
        Self {
            position,
            hexes: vec![Hex {
                kind: HexType::LastMove,
                position: position,
                level: 0,
            }],
        }
    }

    pub fn add_target(&mut self) {
        if self.hexes.iter().any(|hex| hex.kind == HexType::LastMove ) {
            self.hexes.push(Hex {
                kind: HexType::Target,
                position: self.position,
                level: self.hexes.len() - 1,
            })
        } else {
            self.hexes.push(Hex {
                kind: HexType::Target,
                position: self.position,
                level: self.hexes.len() + 1,
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
