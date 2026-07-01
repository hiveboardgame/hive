use hudsoni::{Bug, Color, Piece};

use crate::game::Game;

pub const WIN: i32 = 1_000_000;
pub const INF: i32 = 2_000_000;

pub fn evaluate(game: &Game) -> i32 {
    let us = game.turn_color;
    let them = us.opposite_color();
    (queen_neighbors(game, them) - queen_neighbors(game, us)) * 10
}

fn queen_neighbors(game: &Game, color: Color) -> i32 {
    let queen = Piece::new_from(Bug::Queen, color, 0);
    game.board
        .position_of_piece(queen)
        .map(|pos| i32::from(*game.board.neighbor_count.get(pos)))
        .unwrap_or(0)
}
