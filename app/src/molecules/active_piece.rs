use crate::common::game_state::GameState;
use crate::common::piece_type::PieceType;
use crate::atoms::piece::Piece;
use hive_lib::piece::Piece;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn ActivePiece(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
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

    let level = move || match game_state.get().position.get() {
        Some(pos) => game_state.get().state.get().board.board.get(pos).len(),
        None => 0,
    };

    // Show active piece
    view! {cx,
        {
            move || if active_piece() {
                log!("Showing active piece: {:?}", piece());
                view! {cx, <Piece piece=piece position=position level=level() piece_type=PieceType::Spawn/> }
            } else {
                log!("No active piece");
                view! {cx, {}}.into_view(cx)
            }
        }
    }
}
