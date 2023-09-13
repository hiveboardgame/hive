use crate::common::piece_type::PieceType;
use hive_lib::piece::Piece;
use hive_lib::position::Position;

#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    From,
    To,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HexType {
    // Show Active piece
    Active,
    // Last made move
    LastMove,
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
