use crate::common::piece_type::PieceType;
use crate::atoms::piece::Piece;
use hive_lib::{
    piece::Piece, position::Position,
};
use leptos::*;

#[component]
pub fn PieceStack(cx: Scope, pieces: Vec<(Piece, Position, PieceType)>) -> impl IntoView {
    let len = pieces.len() - 1;
    let onclick = move |_| log!("piece stack");
    pieces
        .into_iter()
        .enumerate()
        .map(|(i, (piece, position, piece_type))| {
            let mut piecetype = PieceType::Covered;
            if i == len {
                piecetype = piece_type;
            };
            view! {cx,
                <Piece on:click=onclick piece=piece position=position level=i piece_type=piecetype/>
            }
        })
        .collect_view(cx)
}
