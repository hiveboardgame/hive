use anyhow::{anyhow, Result};
use hive_lib::{Color, GameResult, GameStatus, GameType, State};
use std::str::FromStr;

/// The game after `moves`, by the site's rules. A record that played on through a repetition
/// is not refereed again here.
fn replay(game_type: &str, moves: &[String]) -> Result<State> {
    let game_type = GameType::from_str(game_type).map_err(|e| anyhow!("{game_type}: {e}"))?;
    let queen_opened = moves
        .iter()
        .take(2)
        .any(|mv| mv.starts_with("wQ") || mv.starts_with("bQ"));
    let mut state = State::new(game_type, !queen_opened);
    for (ply, mv) in moves.iter().enumerate() {
        state
            .replay_turns([split(mv)])
            .map_err(|e| anyhow!("replaying move {} ({mv}): {e}", ply + 1))?;
    }
    Ok(state)
}

fn split(mv: &str) -> (&str, &str) {
    mv.split_once(' ').unwrap_or((mv, ""))
}

/// How the game stood after its last move, by the site's rules, as the win chance of the side
/// to move: won 100, drawn 50, lost 0, or None if the game did not end on the board. The
/// engine's idea of a repetition may differ from Hive's: after a throw, the stunned piece
/// makes the position a different one.
pub fn ending(game_type: &str, moves: &[String]) -> Result<Option<f32>> {
    let mut state = replay(game_type, moves)?;
    state.finish_repetition_at_final_ply();
    // Not `state.turn_color`: a move that ends the game leaves it on the mover.
    let to_move = if moves.len() % 2 == 0 {
        Color::White
    } else {
        Color::Black
    };
    Ok(match state.game_status {
        GameStatus::Finished(GameResult::Draw) => Some(50.0),
        GameStatus::Finished(GameResult::Winner(winner)) if winner == to_move => Some(100.0),
        GameStatus::Finished(GameResult::Winner(_)) => Some(0.0),
        _ => None,
    })
}

/// Whether `a` and `b`, played after `moves`, lead to the same position: UHP can spell one move
/// several ways. A move the rules refuse is the same as nothing.
pub fn same_move(game_type: &str, moves: &[String], a: &str, b: &str) -> Result<bool> {
    let state = replay(game_type, moves)?;
    let after = |mv: &str| {
        let mut state = state.clone();
        state
            .replay_turns([split(mv)])
            .ok()
            .and_then(|()| state.hashes.last().copied())
    };
    Ok(match (after(a), after(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moves(text: &str) -> Vec<String> {
        text.split(';')
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(String::from)
            .collect()
    }

    #[test]
    fn a_surrounded_queen_ends_the_game_for_the_side_to_move() {
        let game = moves(
            r"wA1;bA1 wA1-;wA2 /wA1;bQ bA1\;wQ \wA2;bG1 bQ/;wM wQ/;bG1 -wQ;wM -bQ;bM bQ/;
              wG1 /wM;bM wQ/;wL -wG1;bA1 /wQ;wL /bQ;bL bM/;wG1 bQ\;bL \wQ",
        );
        assert_eq!(ending("Base+ML", &game).unwrap(), Some(0.0));
        assert_eq!(ending("Base+ML", &game[..17]).unwrap(), None);
    }

    #[test]
    fn a_game_that_stops_on_its_third_repetition_is_drawn() {
        let shuffle =
            r"wL;bL wL-;wQ /wL;bQ bL/;wQ -wL;bQ bL-;wQ /wL;bQ bL/;wQ -wL;bQ bL-;wQ /wL;bQ bL/";
        assert_eq!(ending("Base+MLP", &moves(shuffle)).unwrap(), Some(50.0));
        let played_on = moves(&format!("{shuffle};wA1 -wQ"));
        assert_eq!(ending("Base+MLP", &played_on).unwrap(), None);
    }

    #[test]
    fn a_throw_makes_a_new_position_where_an_engine_may_see_a_repetition() {
        let fixture = include_str!("../../engine/test_pgns/regressions/false_draw_own_throws.uhp");
        let game: Vec<String> = moves(fixture).into_iter().skip(3).collect();
        assert_eq!(game.len(), 39);
        assert_eq!(ending("Base+MLP", &game).unwrap(), None);
    }

    #[test]
    fn a_move_the_rules_refuse_is_an_error() {
        assert!(ending("Base+MLP", &moves("wL;wQ wL-")).is_err());
    }

    #[test]
    fn two_spellings_of_one_move_are_the_same_move() {
        // The local game that exposed this: its 17th move, written two ways.
        let game = moves(
            r"wL;bL wL-;wP /wL;bQ bL/;wQ \wP;bM bQ\;wA1 \wL;bP bM/;wA1 \bP;bA1 bL\;
              wG1 \wA1;bA2 bM\;wB1 -wG1;bA2 -wB1;wG2 wG1/;bA3 bM\",
        );
        assert!(same_move("Base+MLP", &game, r"wG2 wB1\", "wG2 -wA1").unwrap());
        assert!(!same_move("Base+MLP", &game, r"wG2 wB1\", "wB2 -wA1").unwrap());
        assert!(!same_move("Base+MLP", &game, r"wG2 wB1\", "wQ -bQ").unwrap());
    }
}
