use crate::common::{game_state::GameState, piece_type::PieceType};
use crate::molecules::piece_stack::PieceStack;

use hive_lib::piece::Piece;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn BoardPieces(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
    let state = move || game_state.get().state.get();
    let display = move || state().get_board().display_iter();

    let board_pieces = move || {
        let mut found = Vec::new();
        for (pos, bug_stack) in display() {
            found.push(
                (0..bug_stack.len())
                    .map(|i| (bug_stack.pieces[i], pos, PieceType::Board))
                    .collect::<Vec<(Piece, Position, PieceType)>>(),
            );
        }
        found
    };

    let bv = move || board_pieces()
        .into_iter()
        .map(|p| {
            view! { cx, <PieceStack pieces=p/> }
        })
        .collect_view(cx);

    view! {cx, {bv }}
    // view! {cx,
    //     <For
    //     each=board_pieces
    //     key=|pieces| (pieces.last().unwrap().0.to_string())
    //     view=move |cx, pieces: (Vec<(Piece, Position, PieceType)>)| {
    //         view! {cx, <PieceStack pieces=pieces/>}
    //     }
    //   />
    // }
}
