use crate::atoms::svgs::Svgs;
use crate::common::game_state::{GameState};
use crate::common::piece_type::{PieceType};
use crate::molecules::{piece::Piece, piece_stack::PieceStack, target::Target};

use hive_lib::piece::Piece;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Board(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");

    let targets = move || game_state.get().target_positions.get();

    let piece = MaybeSignal::derive(cx, move || {
        game_state.get().active.get().unwrap_or(Piece::new())
    });
    let position = MaybeSignal::derive(cx, move || {
        game_state
            .get()
            .position
            .get()
            .unwrap_or(Position::new(0, 0))
    });
    let active_piece = move || {
        game_state.get().active.get().is_some() && game_state.get().position.get().is_some()
    };

    // Show active piece
    let active_piece_view = view! {cx,
        {
            move || if active_piece() {
                log!("Showing active piece: {:?}", piece());
                view! {cx, <Piece piece=piece position=position level=0 piece_type=PieceType::Spawn/> }
            } else {
                log!("No active piece");
                view! {cx, {}}.into_view(cx)
            }
        }

    };

    // let spawn_positions = move || game_state.get().target_positions.get();
    // Show target positions
    let target_view = view! {cx,
        <For
        // a function that returns the items we're iterating over; a signal is fine
        each=targets
        // a unique key for each item
        key=|target| (target.q, target.r)
        // renders each item to a view
        view=move |cx, target: Position| {
            view! {cx, <Target position=target level=0/>}
        }
      />
    };

    let mut board = Vec::new();
    for (pos, bug_stack) in game_state.get().state.get().board.display_iter() {
        board.push(
            (0..bug_stack.len())
                .map(|i| (bug_stack.pieces[i], pos, PieceType::Board))
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
            { target_view }
            { active_piece_view }
            // { spawns_view }
            { pieces_view }
        </svg>
    }
}
