use crate::{
    common::HexStack,
    components::molecules::hex_stack::HexStack as HexStackView,
    providers::{config::TileOptions, game_state::GameStateSignal},
};
use hive_lib::{History, Position, State};
use leptos::prelude::*;

#[component]
pub fn HistoryPieces(
    tile_opts: TileOptions,
    target_stack: RwSignal<Option<Position>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();

    let history_turn = create_read_slice(game_state.signal, |gs| gs.history_turn);
    let history_moves = create_read_slice(game_state.signal, |gs| gs.state.history.moves.clone());

    let history_state = Memo::new(move |_| {
        history_moves.with(|moves| {
            let mut history = History::new();
            if let Some(turn) = history_turn() {
                if turn < moves.len() {
                    history.moves = moves[0..=turn].into();
                }
            }
            State::new_from_history(&history).expect("Got state from history")
        })
    });

    let history_pieces = move || {
        history_state.with(|state| {
            let mut pieces = Vec::new();
            for r in 0..32 {
                for q in 0..32 {
                    let position = Position::new(q, r);
                    let bug_stack = state.board.board.get(position);
                    if !bug_stack.is_empty() {
                        pieces.push(HexStack::new_history(bug_stack, position));
                    }
                }
            }
            pieces
        })
    };

    move || {
        history_pieces()
            .into_iter()
            .map(|hs| {
                view! { <HexStackView hex_stack=hs tile_opts=tile_opts.clone() target_stack /> }
            })
            .collect_view()
    }
}
