use crate::common::piece_type::PieceType;
use crate::molecules::piece::Piece;
use crate::common::svg_pos::SvgPos;
use hive_lib::{
    board::Board, bug::Bug, color::Color, game_type::GameType, piece::Piece, position::Position,
};
use leptos::*;

#[component]
pub fn PieceStack(cx: Scope, pieces: Vec<(Piece, Position, PieceType)>) -> impl IntoView {
    let len = pieces.len() - 1;

    let stack = pieces
        .into_iter()
        .enumerate()
        .map(|(i, (piece, position, piece_type))| {
            let piecetype = if i == len {
                piece_type.clone()
            } else {
                PieceType::Covered
            };
            view! {cx,
                <Piece piece=piece position=position level=i piece_type=piecetype/>
            }
        })
        .collect_view(cx);

    view! {cx,
        { stack }
    }
}
