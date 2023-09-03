use crate::common::piece_type::PieceType;
use hive_lib::position::Position;
use hive_lib::piece::Piece;

pub enum HexType {
    // The Game piece and its type
    Tile(Piece, PieceType),
    // spawn or move
    Target, 
    // Last made move
    LastMove,
}

pub struct Hex {
    pub kind: HexType,
    pub position: Position,
    pub level: usize,
}
