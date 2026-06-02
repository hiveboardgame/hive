use crate::providers::game_state::GameStateSignal;
use hive_lib::{History, State};
use leptos::prelude::*;

fn build_history_state(history_moves: &[(String, String)], history_turn: Option<usize>) -> State {
    let mut history = History::new();
    if let Some(turn) = history_turn {
        if turn < history_moves.len() {
            history.moves = history_moves[0..=turn].into();
        }
    }
    State::new_from_history(&history).expect("Got state from history")
}

pub fn selected_history_state(game_state: GameStateSignal) -> Memo<State> {
    let history_turn = create_read_slice(game_state.signal, |gs| gs.history_turn);
    let history_moves = create_read_slice(game_state.signal, |gs| gs.state.history.moves.clone());

    Memo::new(move |_| history_moves.with(|moves| build_history_state(moves, history_turn())))
}
