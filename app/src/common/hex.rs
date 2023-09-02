use crate::common::piece_type::PieceType;
use hive_lib::position::Position;
use hive_lib::piece::Piece;

pub enum HexType {
    Piece,
    Destination,
    LastMove,
}

pub struct Hex {
    pub kind: HexType,
    pub piece: Option<Piece>,
    pub position: Position,
    pub piece_type: PieceType,
    pub level: usize,
}
