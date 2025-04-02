use crate::{
    common::HexStack,
    components::molecules::hex_stack::HexStack as HexStackView,
    pages::play::TargetStack,
    providers::{game_state::GameStateSignal, Config},
};
use hive_lib::{History, Position, State};
use leptos::prelude::*;

#[component]
pub fn HistoryPieces() -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let config = expect_context::<Config>().0;
    let target_stack = expect_context::<TargetStack>().0;
    let tile_opts = Signal::derive(move || config().tile);
    let history_pieces = move || {
        let mut history_pieces = Vec::new();
        let game_state = (game_state_signal.signal)();
        let mut history = History::new();
        //log!("history_turn: {:?}", game_state.history_turn);
        if let Some(turn) = game_state.history_turn {
            history.moves = game_state.state.history.moves[0..=turn].into();
        }
        let state = State::new_from_history(&history).expect("Got state from history");
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                let bug_stack = state.board.board.get(position).clone();
                if !bug_stack.is_empty() {
                    history_pieces.push(HexStack::new_history(&bug_stack, position));
                }
            }
        }
        history_pieces
    };

    move || {
        history_pieces()
            .into_iter()
            .map(|hs| {
                view! { <HexStackView hex_stack=hs tile_opts=tile_opts() target_stack /> }
            })
            .collect_view()
    }
}
