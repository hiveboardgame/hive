use crate::providers::game_state::{BoardView, GameStateStore, GameStateStoreFields};
use hive_lib::{GameType, History, State};
use leptos::prelude::*;

fn build_history_state(
    history_moves: &[(String, String)],
    history_turn: Option<usize>,
    game_type: GameType,
) -> State {
    let mut history = History::new();
    history.game_type = game_type;
    if let Some(turn) = history_turn {
        if turn < history_moves.len() {
            history.moves = history_moves[0..=turn].into();
        }
    }
    State::new_from_history(&history).expect("Got state from history")
}

pub fn selected_history_state(game_state: GameStateStore) -> Memo<State> {
    let board_view = game_state.board_view();
    let state = game_state.state();
    let history =
        Memo::new(move |_| state.with(|state| (state.history.moves.clone(), state.game_type)));

    Memo::new(move |_| {
        let board_view = board_view.get();
        history.with(|(moves, game_type)| {
            let history_turn = match board_view {
                BoardView::Live => moves.len().checked_sub(1),
                BoardView::History { turn } => turn,
            };
            build_history_state(moves, history_turn, *game_type)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_history_state_reacts_to_board_view_and_preserves_the_game_type() {
        let owner = Owner::new();
        owner.with(|| {
            let history = History::from_pgn_str(
                include_str!("../../../engine/test_pgns/valid/p_game.pgn").to_string(),
            )
            .expect("valid history");
            let game_state = GameStateStore::new();
            let full_state = State::new_from_history(&history).expect("valid state");
            let canonical_moves = full_state.history.moves.clone();
            game_state.reset_with_state(full_state);

            let selected_state = selected_history_state(game_state);
            let live_state = selected_state.get_untracked();
            assert_eq!(live_state.game_type, GameType::P);
            assert_eq!(live_state.history.moves, canonical_moves);

            let history_turn = 7;
            game_state.show_history_turn(history_turn);

            let actual = selected_state.get_untracked();
            let mut expected_history = History::new();
            expected_history.game_type = history.game_type;
            expected_history.moves = canonical_moves[..=history_turn].to_vec();
            let expected = State::new_from_history(&expected_history).expect("valid history state");

            assert_eq!(actual, expected);
            assert_eq!(actual.game_type, GameType::P);
            assert_eq!(actual.history.moves, canonical_moves[..=history_turn]);
        });
    }
}
