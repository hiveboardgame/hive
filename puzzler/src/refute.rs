use crate::{
    game::Game,
    notation,
    prove::{OutOfBudget, Prover},
};
use hive_lib::State;

/// After a wrong move in a puzzle: the defender's answer, in the site's notation, that leaves the
/// solver no forced win in the `solver_moves_left` moves the puzzle still allowed.
pub fn refutation(
    state: &State,
    solver_moves_left: u32,
    budget: u64,
) -> Result<Option<String>, OutOfBudget> {
    let mut game = Game::from_state(state);
    let plies = (2 * solver_moves_left).saturating_sub(1);
    let mut prover = Prover::new(budget);
    Ok(prover
        .refutation(&mut game, plies)?
        .map(|action| notation::notation(state, &action)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::{play_token, start_state, PuzzleImport};

    #[test]
    fn a_wrong_first_move_gets_an_answer_that_stops_the_win() {
        let puzzle: PuzzleImport =
            serde_json::from_str(include_str!("../../shared_types/test_data/puzzle.json"))
                .expect("fixture puzzle");
        let mut state =
            start_state(&puzzle.game_type, puzzle.history.as_deref(), None).expect("start");
        let solver = state.turn_color;
        let mut game = Game::from_state(&state);
        let mut prover = Prover::new(5_000_000);
        let winning = prover.winning_moves(&mut game, 3).expect("budget");
        let mut actions = Vec::new();
        game.actions(&mut actions);
        let wrong = actions
            .into_iter()
            .find(|action| !winning.contains(action))
            .expect("some move that does not force the win");
        let token = notation::notation(&state, &wrong);
        play_token(&mut state, &token).expect("wrong move plays");
        assert_ne!(state.turn_color, solver);
        let answer = refutation(&state, 1, 5_000_000).expect("budget");
        assert!(
            answer.is_some(),
            "a defence exists against a non-winning move"
        );
    }
}
