use crate::common::piece_type::PieceType;
use hive_lib::{Piece,Position};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    From,
    To,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum ActiveState {
    Board,
    #[default]
    Reserve,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HexType {
    // Show Active piece
    Active(ActiveState),
    // Last made move
    LastMove(Direction),
    // spawn or move spot
    Target,
    // The Game piece and its type
    Tile(Piece, PieceType),
}

#[derive(Debug)]
pub struct Hex {
    pub kind: HexType,
    pub position: Position,
    pub level: usize,
}
