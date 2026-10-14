use crate::game::{Action, Game};
use anyhow::{Context, Result};
use hive_lib::Bug;
use shared_types::{start_state, PuzzleImport};

/// How a puzzle fares against the "no boring wins in one" rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Keep,
    /// A win in one that is just "fill the Queen's last gap".
    Plain,
}

/// Win-in-one puzzles stay only when the player missed the win in the real game, or when every
/// winning move is a trick: a Pillbug throw, or a Mosquito climbing, or leaping like a Grasshopper
/// or Ladybug it copies where no sliding copy could reach. A
/// single ordinary winning move would make the puzzle trivial, since any surround is accepted.
pub fn verdict(puzzle: &mut PuzzleImport) -> Result<Verdict> {
    if shared_types::solver_moves(&puzzle.solution) != 1
        || puzzle.themes.iter().any(|theme| theme == "missedWin")
    {
        return Ok(Verdict::Keep);
    }
    let state = start_state(
        &puzzle.game_type,
        puzzle.history.as_deref(),
        puzzle.position.as_deref(),
    )
    .context("puzzle start")?;
    let game = Game::from_state(&state);
    let mut wins = Vec::new();
    game.immediate_wins(&mut wins);
    let tricks: Option<Vec<&'static str>> = wins.iter().map(|win| trick(&game, win)).collect();
    match tricks {
        Some(tricks) if !tricks.is_empty() => {
            for theme in tricks {
                if !puzzle.themes.iter().any(|existing| existing == theme) {
                    puzzle.themes.push(theme.to_string());
                }
            }
            Ok(Verdict::Keep)
        }
        _ => Ok(Verdict::Plain),
    }
}

fn trick(game: &Game, action: &Action) -> Option<&'static str> {
    let Action::Move { piece, from, to } = action else {
        return None;
    };
    if piece.color() != game.to_move {
        return Some("throw");
    }
    if piece.bug() != Bug::Mosquito {
        return None;
    }
    let board = &game.board;
    if board.occupied(*to) || board.level(*from) > 1 {
        return Some("mosquito");
    }
    let reaches = |bug: Bug| match bug {
        Bug::Ant => Bug::ant_moves(*from, board).contains(to),
        Bug::Spider => Bug::spider_moves(*from, board).contains(to),
        Bug::Queen => Bug::queen_moves(*from, board).any(|cell| cell == *to),
        Bug::Pillbug => Bug::pillbug_moves(*from, board).any(|cell| cell == *to),
        Bug::Grasshopper => Bug::grasshopper_moves(*from, board).contains(to),
        Bug::Ladybug => Bug::ladybug_moves(*from, board).contains(to),
        Bug::Beetle => Bug::beetle_moves(*from, board).contains(to),
        Bug::Mosquito => false,
    };
    let copied: Vec<Bug> = from
        .positions_around()
        .filter_map(|cell| board.top_bug(cell))
        .collect();
    let slides = copied.iter().any(|bug| {
        matches!(bug, Bug::Ant | Bug::Spider | Bug::Queen | Bug::Pillbug) && reaches(*bug)
    });
    let leaps = copied
        .iter()
        .any(|bug| matches!(bug, Bug::Grasshopper | Bug::Ladybug) && reaches(*bug));
    (leaps && !slides).then_some("mosquito")
}

/// The warm-up set: plain wins in one on the smallest boards, a few per finishing bug so they
/// do not all look alike.
pub fn warm_ups(plain: Vec<PuzzleImport>, count: usize, rating: f64) -> Vec<PuzzleImport> {
    let mut plain = plain;
    plain.sort_by_key(|puzzle| {
        puzzle
            .history
            .as_deref()
            .map_or(usize::MAX, |history| history.matches(';').count())
    });
    let per_bug = count.div_ceil(4).max(1);
    let mut taken: Vec<PuzzleImport> = Vec::new();
    for mut puzzle in plain {
        if taken.len() == count {
            break;
        }
        let bug = finishing_bug(&puzzle);
        if taken
            .iter()
            .filter(|other| finishing_bug(other) == bug)
            .count()
            >= per_bug
        {
            continue;
        }
        puzzle.rating = Some(rating);
        puzzle.themes.push("easy".to_string());
        taken.push(puzzle);
    }
    taken
}

fn finishing_bug(puzzle: &PuzzleImport) -> Option<char> {
    puzzle.solution.first()?.chars().nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PuzzleImport {
        serde_json::from_str(include_str!("../../shared_types/test_data/puzzle.json"))
            .expect("fixture puzzle")
    }

    #[test]
    fn longer_puzzles_and_missed_wins_always_stay() {
        let mut longer = fixture();
        assert_eq!(verdict(&mut longer).expect("verdict"), Verdict::Keep);
    }

    #[test]
    fn a_plain_finish_in_one_is_dropped_unless_it_was_missed() {
        let mut plain = fixture();
        let history = plain.history.take().expect("history");
        plain.history = Some(format!(
            "{history}{};{};",
            plain.solution[0], plain.solution[1]
        ));
        plain.solution = vec![plain.solution[2].clone()];
        assert_eq!(
            verdict(&mut plain.clone()).expect("verdict"),
            Verdict::Plain
        );
        plain.themes.push("missedWin".to_string());
        assert_eq!(verdict(&mut plain).expect("verdict"), Verdict::Keep);
    }
}

/// A first move is quiet when it neither lands next to the enemy Queen nor threatens to win on
/// the very next move: the win comes from the setup, not from an obvious threat.
pub fn quiet_first_move(start: &Game, first: &Action) -> bool {
    let solver = start.to_move;
    let beside_queen = first
        .to()
        .zip(start.queen_position(solver.opposite_color()))
        .is_some_and(|(to, queen)| to.is_neighbor(queen));
    if beside_queen {
        return false;
    }
    let mut game = start.clone();
    game.make(first);
    game.make(&Action::Pass);
    let mut wins = Vec::new();
    game.immediate_wins(&mut wins);
    wins.is_empty()
}

/// Recomputes the `quietMove` tag from the start and the first solver move; true when it changed.
pub fn retag_quiet(puzzle: &mut PuzzleImport) -> Result<bool> {
    let was_quiet = puzzle.themes.iter().any(|theme| theme == "quietMove");
    let quiet = shared_types::solver_moves(&puzzle.solution) > 1 && {
        let state = start_state(
            &puzzle.game_type,
            puzzle.history.as_deref(),
            puzzle.position.as_deref(),
        )
        .context("puzzle start")?;
        let first = puzzle.solution.first().context("empty solution")?;
        let (piece, to) = shared_types::resolve_move(&state, first)
            .map_err(anyhow::Error::msg)?
            .context("first solution move is a pass")?;
        let action = match state.board.position_of_piece(piece) {
            Some(from) => Action::Move { piece, from, to },
            None => Action::Place { piece, to },
        };
        quiet_first_move(&Game::from_state(&state), &action)
    };
    if quiet == was_quiet {
        return Ok(false);
    }
    if quiet {
        puzzle.themes.push("quietMove".to_string());
    } else {
        puzzle.themes.retain(|theme| theme != "quietMove");
    }
    Ok(true)
}
