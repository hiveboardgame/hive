use crate::atoms::svgs::Svgs;
use crate::common::game_state::GameState;
use crate::common::piece_type::PieceType;
use crate::molecules::piece_stack::PieceStack;
use hive_lib::piece::Piece;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Board(cx: Scope) -> impl IntoView {
    let game_state = use_context::<RwSignal<GameState>>(cx)
        .expect("there to be a `GameState` signal provided");

    let mut board = Vec::new();
    for (pos, bug_stack) in game_state
        .get()
        .state
        .get()
        .board
        .display_iter()
    {
        board.push(
            (0..bug_stack.len())
                .map(|i| (bug_stack.pieces[i].clone(), pos, PieceType::Board))
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
            // { spawns }
            { pieces_view }
        </svg>
    }
}
