use shared_types::{MoveEval, MAX_EVAL_LINE};

/// The engine's view of one position, from the side to move.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionEval {
    pub winprob: f32,
    pub best: String,
    pub line: Vec<String>,
}

/// None for a position with nothing to search: the game is over or the only move is a pass.
pub type Searched = Option<PositionEval>;

/// How the game is searched: everything at `screen` sims, then the moves that look like they
/// lost at least `threshold` points are searched again at `full` sims. Measured on 13 corpus
/// games, 50 > 2 > 800 grades like a plain 800-sim search at about 80% of its cost; a screen
/// below 50 sims misses about one real mistake in six. The default screen is 64, StockBee's
/// author's suggestion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheme {
    pub screen: u32,
    pub threshold: f32,
    pub full: u32,
}

impl Scheme {
    pub fn label(&self) -> String {
        format!("{}>{}>{}", self.screen, self.threshold, self.full)
    }
}

/// Win chance the mover kept after playing into `after`.
fn kept(after: &Searched) -> f32 {
    match after {
        Some(eval) => 100.0 - eval.winprob,
        // No search after the move: the move ended the game.
        None => 100.0,
    }
}

/// Positions to search again at full sims, given the screen of every position (0..=moves).
pub fn positions_to_recheck(moves: &[String], screen: &[Searched], threshold: f32) -> Vec<usize> {
    let mut positions = Vec::new();
    for (ply, played) in moves.iter().enumerate() {
        let Some(before) = &screen[ply] else { continue };
        if &before.best != played && before.winprob - kept(&screen[ply + 1]) >= threshold {
            positions.extend([ply, ply + 1]);
        }
    }
    positions.sort_unstable();
    positions.dedup();
    positions
}

/// One MoveEval per move. A move is judged at full sims when both the position before and
/// after it were searched at full sims, otherwise at screen sims, so before and after always
/// come from searches of the same depth.
pub fn assemble(
    moves: &[String],
    screen: &[Searched],
    full: &[Option<Searched>],
    scheme: &Scheme,
) -> Vec<Option<MoveEval>> {
    moves
        .iter()
        .enumerate()
        .map(|(ply, played)| {
            let (before, after, sims) = match (&full[ply], &full[ply + 1]) {
                (Some(before), Some(after)) => (before, after, scheme.full),
                _ => (&screen[ply], &screen[ply + 1], scheme.screen),
            };
            let before = before.as_ref()?;
            let mut line: Vec<String> = before.line.iter().take(MAX_EVAL_LINE).cloned().collect();
            if line.first() != Some(&before.best) {
                line = vec![before.best.clone()];
            }
            Some(MoveEval {
                played: played.clone(),
                best: before.best.clone(),
                line,
                before: before.winprob,
                after: kept(after),
                sims,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEME: Scheme = Scheme {
        screen: 50,
        threshold: 2.0,
        full: 800,
    };

    fn pos(winprob: f32, best: &str) -> Searched {
        Some(PositionEval {
            winprob,
            best: best.to_string(),
            line: vec![best.to_string(), "reply".to_string()],
        })
    }

    fn moves(list: &[&str]) -> Vec<String> {
        list.iter().map(|m| m.to_string()).collect()
    }

    #[test]
    fn a_move_that_loses_enough_is_rechecked_with_both_of_its_positions() {
        let mv = moves(&["wL", "bG1 wL/"]);
        // White keeps 100 - 45 = 55 of 65: a 10 point drop.
        let screen = vec![pos(65.0, "wP"), pos(45.0, "bP wL/"), pos(50.0, "wM /wL")];
        assert_eq!(positions_to_recheck(&mv, &screen, 2.0), vec![0, 1]);
    }

    #[test]
    fn the_engines_own_choice_is_never_rechecked() {
        let mv = moves(&["wL"]);
        let screen = vec![pos(65.0, "wL"), pos(10.0, "bP wL/")];
        assert!(positions_to_recheck(&mv, &screen, 2.0).is_empty());
    }

    #[test]
    fn a_small_drop_is_not_rechecked() {
        let mv = moves(&["wL"]);
        let screen = vec![pos(65.0, "wP"), pos(36.0, "bP wL/")];
        assert!(positions_to_recheck(&mv, &screen, 2.0).is_empty());
    }

    #[test]
    fn a_move_that_ends_the_game_keeps_everything() {
        let mv = moves(&["wQ -bQ"]);
        let screen = vec![pos(90.0, "wA1 bQ/"), None];
        let result = assemble(&mv, &screen, &[None, None], &SCHEME);
        let eval = result[0].as_ref().unwrap();
        assert_eq!(eval.after, 100.0);
        assert_eq!(eval.loss(), 0.0);
    }

    #[test]
    fn a_rechecked_move_is_judged_at_full_sims() {
        let mv = moves(&["wL", "bG1 wL/"]);
        let screen = vec![pos(65.0, "wP"), pos(45.0, "bP wL/"), pos(50.0, "wM /wL")];
        let full = vec![Some(pos(64.0, "wP")), Some(pos(48.0, "bP wL/")), None];
        let result = assemble(&mv, &screen, &full, &SCHEME);

        let first = result[0].as_ref().unwrap();
        assert_eq!((first.before, first.after, first.sims), (64.0, 52.0, 800));
        // The second move's after-position only has a screen search, so it stays at 50 sims.
        let second = result[1].as_ref().unwrap();
        assert_eq!((second.before, second.after, second.sims), (45.0, 50.0, 50));
    }

    #[test]
    fn a_position_with_nothing_to_search_gives_no_judgement() {
        let mv = moves(&["pass"]);
        let result = assemble(&mv, &[None, pos(50.0, "wA1 -bQ")], &[None, None], &SCHEME);
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn long_lines_are_cut_to_what_the_server_accepts() {
        let mv = moves(&["wL"]);
        let mut first = pos(65.0, "wP").unwrap();
        first.line = std::iter::once("wP".to_string())
            .chain((0..40).map(|i| format!("m{i}")))
            .collect();
        let result = assemble(&mv, &[Some(first), pos(40.0, "x")], &[None, None], &SCHEME);
        assert_eq!(result[0].as_ref().unwrap().line.len(), MAX_EVAL_LINE);
    }

    #[test]
    fn a_line_not_led_by_the_best_move_is_replaced_by_the_best_move() {
        let mv = moves(&["wL"]);
        let mut first = pos(65.0, "wP").unwrap();
        first.line = vec!["wG1".to_string()];
        let result = assemble(&mv, &[Some(first), pos(40.0, "x")], &[None, None], &SCHEME);
        assert_eq!(result[0].as_ref().unwrap().line, vec!["wP".to_string()]);
    }
}
