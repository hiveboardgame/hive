use crate::{board::Board, color::Color, moves::Moves, piece::Piece, position::Position};

pub struct RandomAI {
    color: Color,
}

impl RandomAI {
    fn new(color: Color) -> Self {
        RandomAI { color }
    }

    fn random() -> bool {
        true
    }

    fn random_move(number: i32, board: &Board) -> (Piece, Position, Position) {
        let moves = Moves::new(number, &board);
        if RandomAI::random() {
            moves.moves
        }
    }
}
