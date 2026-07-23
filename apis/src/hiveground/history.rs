use crate::providers::game_state::{BoardView, GameStateStore, GameStateStoreFields};
use hive_lib::{Board, GameType, History, State};
use leptos::prelude::*;

fn build_history_board(
    history_moves: &[(String, String)],
    history_turn: Option<usize>,
    game_type: GameType,
) -> Board {
    let mut history = History::new();
    history.game_type = game_type;
    if let Some(turn) = history_turn {
        if turn < history_moves.len() {
            history.moves = history_moves[0..=turn].into();
        }
    }
    State::new_from_history(&history)
        .expect("Got state from history")
        .board
}

fn history_board_for_view(state: &State, board_view: BoardView) -> Board {
    let history_turn = board_view.displayed_turn(state.turn);
    if board_view.is_last_turn(state.turn) {
        state.board.clone()
    } else {
        build_history_board(&state.history.moves, history_turn, state.game_type)
    }
}

pub fn selected_history_board(game_state: GameStateStore) -> Memo<Board> {
    let board_view = game_state.board_view();
    let state = game_state.state();

    Memo::new(move |_| {
        let board_view = board_view.get();
        state.with(|state| history_board_for_view(state, board_view))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_history_board_reacts_to_board_view() {
        let owner = Owner::new();
        owner.with(|| {
            let history =
                History::from_pgn_str(include_str!("../../../engine/test_pgns/valid/p_game.pgn"))
                    .expect("valid history");
            let full_state = State::new_from_history(&history).expect("valid state");
            let game_state = GameStateStore::new();
            game_state.reset_with_state(full_state.clone());

            let selected_board = selected_history_board(game_state);
            assert_eq!(selected_board.get_untracked(), full_state.board);

            let history_turn = 7;
            game_state.show_history_turn(history_turn);
            let mut prefix = history;
            prefix.moves.truncate(history_turn + 1);
            let expected = State::new_from_history(&prefix)
                .expect("valid prefix")
                .board;

            assert_eq!(selected_board.get_untracked(), expected);
        });
    }
}
