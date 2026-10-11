use crate::{
    game::{Action, Game},
    prove::{OutOfBudget, Prover},
};

/// More winning moves than this at one solver turn means the position is not a puzzle, just
/// a won game.
pub const MAX_BRANCHES: usize = 8;
pub const MAX_LINES: usize = 24;

/// A forced win as a puzzle: every line alternates solver and defender moves, solver first. The
/// first line is the main one; the others start with another solver move that wins just as fast.
/// Defender moves are the longest-holding defence. At the final move any surround is accepted, so
/// alternative final moves are not listed.
#[derive(Debug, Clone)]
pub struct Line {
    pub lines: Vec<Vec<Action>>,
    pub plies: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// Too many equally fast wins at some solver turn.
    Ambiguous,
    /// The defence analysis did not reproduce the proven length.
    Inconsistent,
}

/// Extracts the lines of a forced win of exactly `plies` for the side to move.
pub fn solve(
    prover: &mut Prover,
    root: &Game,
    plies: u32,
) -> Result<Result<Line, Rejection>, OutOfBudget> {
    let mut game = root.clone();
    let mut lines = Vec::new();
    let outcome = expand(prover, &mut game, plies, &mut Vec::new(), &mut lines)?;
    Ok(outcome.map(|()| Line { lines, plies }))
}

fn expand(
    prover: &mut Prover,
    game: &mut Game,
    plies: u32,
    prefix: &mut Vec<Action>,
    lines: &mut Vec<Vec<Action>>,
) -> Result<Result<(), Rejection>, OutOfBudget> {
    let winning = prover.winning_moves(game, plies)?;
    let Some(first) = winning.first() else {
        return Ok(Err(Rejection::Inconsistent));
    };
    if plies == 1 {
        let mut line = prefix.clone();
        line.push(*first);
        lines.push(line);
        return Ok(Ok(()));
    }
    if winning.len() > MAX_BRANCHES {
        return Ok(Err(Rejection::Ambiguous));
    }
    for solver_move in winning {
        let solver_undo = game.make(&solver_move);
        let defence = prover.longest_defence(game, plies - 1);
        let outcome = match defence {
            Ok(Some((reply, length))) if length == plies - 2 => {
                let reply_undo = game.make(&reply);
                prefix.extend([solver_move, reply]);
                let outcome = expand(prover, game, length, prefix, lines);
                prefix.truncate(prefix.len() - 2);
                game.unmake(reply_undo);
                outcome
            }
            Ok(_) => Ok(Err(Rejection::Inconsistent)),
            Err(error) => Err(error),
        };
        game.unmake(solver_undo);
        match outcome? {
            Ok(()) if lines.len() <= MAX_LINES => {}
            Ok(()) => return Ok(Err(Rejection::Ambiguous)),
            Err(rejection) => return Ok(Err(rejection)),
        }
    }
    Ok(Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{notation, prove::tests::fixtures};

    #[test]
    fn solved_lines_replay_to_a_win_on_the_site_engine() {
        let mut prover = Prover::new(50_000_000);
        let mut solved = 0;
        for (state, plies) in fixtures().iter().take(30) {
            let game = Game::from_state(state);
            let Ok(Ok(line)) = solve(&mut prover, &game, *plies) else {
                continue;
            };
            let spelled: Vec<Vec<String>> = line
                .lines
                .iter()
                .map(|actions| {
                    let mut replay = state.clone();
                    actions
                        .iter()
                        .map(|action| {
                            let token = notation::notation(&replay, action);
                            notation::play(&mut replay, action).expect("line plays");
                            token
                        })
                        .collect()
                })
                .collect();
            assert_eq!(
                shared_types::verify_lines(state, &spelled),
                Ok(state.turn_color)
            );
            solved += 1;
        }
        assert!(
            solved >= 10,
            "only {solved} fixtures produced a puzzle line"
        );
    }
}
