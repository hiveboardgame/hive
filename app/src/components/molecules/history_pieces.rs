use crate::{
    common::hex_stack::HexStack, components::molecules::hex_stack::HexStack as HexStackView,
    providers::game_state::GameStateSignal,
};
use hive_lib::{history::History, position::Position, state::State};
use leptos::logging::log;
use leptos::*;

#[component]
pub fn HistoryPieces() -> impl IntoView {
    let game_state_signal =
        use_context::<GameStateSignal>().expect("there to be a `GameState` signal provided");

    let history_pieces = move || {
        let mut history_pieces = Vec::new();
        let game_state = game_state_signal.signal.get();
        let mut history = History::new();
        log!("history_turn: {:?}", game_state.history_turn);
        if let Some(turn) = game_state.history_turn {
            history.moves = game_state.state.history.moves[0..=turn].into();
        }
        let state = State::new_from_history(&history).unwrap();
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
                view! {  <HexStackView hex_stack=hs/> }
            })
            .collect_view()
    }
}
