use crate::providers::game_state::{GameStateStore, GameStateStoreFields};
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

pub fn selected_history_state(game_state: GameStateStore) -> Memo<State> {
    let history_turn = game_state.history_turn();
    let state = game_state.state();
    let history_moves = Memo::new(move |_| state.with(|state| state.history.moves.clone()));

    Memo::new(move |_| history_moves.with(|moves| build_history_state(moves, history_turn.get())))
}
