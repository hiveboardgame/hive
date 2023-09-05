use crate::atoms::svgs::Svgs;
use crate::common::piece_type::PieceType;
use crate::molecules::piece_stack::PieceStack;
use hive_lib::bug_stack::BugStack;
use hive_lib::piece::Piece;
use hive_lib::position::Position;
use hive_lib::state::State;
use leptos::*;

#[component]
pub fn Board(cx: Scope, state: State) -> impl IntoView {
    let mut board = Vec::new();
    for (pos, bug_stack) in state.board.display_iter() {
        board.push(
            (0..bug_stack.len())
                .map(|i| {
                    (bug_stack.pieces[i].clone(), pos, PieceType::Board)
                })
                .collect::<Vec<(Piece, Position, PieceType)>>(),
        );
    }
    let pieces_view = board
        .into_iter()
        .map(|p| {
            view! {cx, <PieceStack pieces=p/> }
        })
        .collect_view(cx);
    view! { cx,
        <svg viewBox="1000 450 700 700" style="flex: 1" xmlns="http://www.w3.org/2000/svg">
            <Svgs/>
            { pieces_view }
        </svg>
    }
}
