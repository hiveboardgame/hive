use shared_types::{Grade, MoveEval, MAX_EVAL_LINE};

/// The engine's view of one position, from the side to move.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionEval {
    pub winprob: f32,
    pub best: String,
    pub line: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Searched {
    Position(PositionEval),
    /// The game is over; the side to move has won (100), drawn (50) or lost (0).
    GameOver {
        winprob: f32,
    },
    /// The side to move can only pass, so there is nothing to search.
    OnlyPass,
}

/// How the game is searched: everything with the `screen` budget, then the moves that look like
/// they lost at least `threshold` points, or swing by more than their mark shows, are searched
/// again with the `full` budget (see `positions_to_recheck`). Each move of the line shown after a
/// marked move gets the `line` budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheme {
    pub screen: u32,
    pub threshold: f32,
    pub full: u32,
    pub line: u32,
}

impl Scheme {
    pub fn label(&self) -> String {
        format!(
            "{}>{}>{} line {}",
            self.screen, self.threshold, self.full, self.line
        )
    }
}

/// The position that shows what the move at `ply` left its mover: the one it leads to, or,
/// when that leaves the opponent only a pass, the one after the pass, with the mover to move
/// again.
fn after_position(positions: &[Searched], ply: usize) -> usize {
    match positions.get(ply + 1) {
        Some(Searched::OnlyPass) => ply + 2,
        _ => ply + 1,
    }
}

/// Win chance the mover kept with the move at `ply`, read from `after` at `after_position`;
/// None when that position says nothing, as when the game stops on a forced pass.
fn kept(after: Option<&Searched>, ply: usize, at: usize) -> Option<f32> {
    let to_move = match after? {
        Searched::Position(eval) => eval.winprob,
        Searched::GameOver { winprob } => *winprob,
        Searched::OnlyPass => return None,
    };
    let mover_to_move = at > ply + 1;
    Some(if mover_to_move {
        to_move
    } else {
        100.0 - to_move
    })
}

/// Positions still to search at full sims, given the screen of every position (0..=moves) and
/// the full searches so far. A move gets both of its positions searched when it looks like it
/// lost `threshold` points, or when the swing the graph shows for it would be graded but the
/// move cannot be: the engine keeps its win chance with its own move and never gains any, so
/// such a swing means one of the two positions was misjudged. A position the full search
/// corrects can expose the move before it, so this is asked again until it finds nothing new.
pub fn positions_to_recheck(
    moves: &[String],
    screen: &[Searched],
    full: &[Option<Searched>],
    threshold: f32,
) -> Vec<usize> {
    let known = |p: usize| {
        full.get(p)
            .and_then(Option::as_ref)
            .or_else(|| screen.get(p))
    };
    let unchecked = |p: &usize| full.get(*p).is_none_or(Option::is_none);
    let mut positions = Vec::new();
    for (ply, played) in moves.iter().enumerate() {
        let Some(Searched::Position(before)) = known(ply) else {
            continue;
        };
        let at = after_position(screen, ply);
        let Some(kept) = kept(known(at), ply, at) else {
            continue;
        };
        let change = before.winprob - kept;
        let own = &before.best == played;
        let lost = !own && change >= threshold;
        let marked = Grade::from_loss(if own { 0.0 } else { change.max(0.0) });
        if lost || Grade::from_loss(change.abs()) != marked {
            positions.extend([ply, at].into_iter().filter(unchecked));
        }
    }
    positions.sort_unstable();
    positions.dedup();
    positions
}

/// One MoveEval per move. Every position counts at its deepest search, so a move's before is
/// the previous move's after and the graph swings only where a move does. A move that could
/// be graded had both positions rechecked, so grades always come from full sims; `sims` is
/// the shallower of the two searches.
pub fn assemble(
    moves: &[String],
    screen: &[Searched],
    full: &[Option<Searched>],
    scheme: &Scheme,
) -> Vec<Option<MoveEval>> {
    let rechecked = |p: usize| full.get(p).and_then(Option::as_ref);
    let known = |p: usize| rechecked(p).or_else(|| screen.get(p));
    moves
        .iter()
        .enumerate()
        .map(|(ply, played)| {
            let at = after_position(screen, ply);
            let Searched::Position(before) = known(ply)? else {
                return None;
            };
            let after = kept(known(at), ply, at)?;
            let sims = if rechecked(ply).is_some() && rechecked(at).is_some() {
                scheme.full
            } else {
                scheme.screen
            };
            let mut line: Vec<String> = before.line.iter().take(MAX_EVAL_LINE).cloned().collect();
            if line.first() != Some(&before.best) {
                line = vec![before.best.clone()];
            }
            Some(MoveEval {
                played: played.clone(),
                best: before.best.clone(),
                line,
                before: before.winprob,
                after,
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
        line: 800,
    };

    fn pos(winprob: f32, best: &str) -> Searched {
        Searched::Position(PositionEval {
            winprob,
            best: best.to_string(),
            line: vec![best.to_string(), "reply".to_string()],
        })
    }

    fn over(winprob: f32) -> Searched {
        Searched::GameOver { winprob }
    }

    fn moves(list: &[&str]) -> Vec<String> {
        list.iter().map(|m| m.to_string()).collect()
    }

    fn unchecked(positions: usize) -> Vec<Option<Searched>> {
        vec![None; positions]
    }

    #[test]
    fn a_move_that_loses_enough_is_rechecked_with_both_of_its_positions() {
        let mv = moves(&["wL", "bG1 wL/"]);
        // White keeps 100 - 45 = 55 of 65: a 10 point drop.
        let screen = vec![pos(65.0, "wP"), pos(45.0, "bP wL/"), pos(54.0, "wM /wL")];
        assert_eq!(positions_to_recheck(&mv, &screen, &[], 2.0), vec![0, 1]);
    }

    #[test]
    fn a_swing_too_small_to_grade_is_not_rechecked_on_the_engines_own_choice() {
        let mv = moves(&["wL"]);
        // 65 before, 100 - 39 = 61 kept.
        let screen = vec![pos(65.0, "wL"), pos(39.0, "bP wL/")];
        assert!(positions_to_recheck(&mv, &screen, &[], 2.0).is_empty());
    }

    #[test]
    fn a_swing_the_size_of_an_inaccuracy_that_no_mark_could_show_is_rechecked() {
        let mv = moves(&["wL"]);
        // The engine's own choice losing 5 (65 to 100 - 40), and a move gaining 5.
        let own = vec![pos(65.0, "wL"), pos(40.0, "bP wL/")];
        assert_eq!(positions_to_recheck(&mv, &own, &[], 2.0), vec![0, 1]);
        let gain = vec![pos(65.0, "wP"), pos(30.0, "bP wL/")];
        assert_eq!(positions_to_recheck(&mv, &gain, &[], 2.0), vec![0, 1]);
    }

    #[test]
    fn a_small_drop_is_not_rechecked() {
        let mv = moves(&["wL"]);
        let screen = vec![pos(65.0, "wP"), pos(36.0, "bP wL/")];
        assert!(positions_to_recheck(&mv, &screen, &[], 2.0).is_empty());
    }

    /// Moves 28 to 32 of a local game, with a 64-sim screen and 800-sim searches.
    /// The screen misjudged the positions after moves 28 and 30, so it saw White's
    /// 31st move gain 27 and Black's 32nd, its own choice, lose 66, and graded neither.
    #[test]
    fn a_misjudged_position_is_followed_back_to_the_move_that_lost_the_game() {
        let mv = moves(&[r"bA2 bA1\", r"wA1 wM\", r"bG2 bB1\", r"wL bQ\", r"bG2 \wQ"]);
        let screen = vec![
            pos(99.2, "bM -bB1"),
            pos(0.2, "wA1 /bM"),
            pos(96.2, r"bL bB1\"),
            pos(1.3, "wA3 /bG2"),
            pos(71.4, r"bG2 \wQ"),
            pos(94.8, r"wA3 \bQ"),
        ];
        let at_800 = [
            pos(99.95, "bM -bB1"),
            pos(21.5, r"wA1 wM\"),
            pos(83.3, "bG1 wA1-"),
            pos(86.0, r"wL bQ\"),
            pos(18.3, "bA2 wB2-"),
            pos(97.0, "wA3 -wA2"),
        ];
        let mut full = unchecked(screen.len());
        let mut passes = Vec::new();
        loop {
            let recheck = positions_to_recheck(&mv, &screen, &full, 2.0);
            if recheck.is_empty() {
                break;
            }
            for &p in &recheck {
                full[p] = Some(at_800[p].clone());
            }
            passes.push(recheck);
        }
        assert_eq!(passes, vec![vec![3, 4, 5], vec![2], vec![1], vec![0]]);
        let grades: Vec<_> = assemble(&mv, &screen, &full, &SCHEME)
            .iter()
            .map(|eval| eval.as_ref().unwrap().grade())
            .collect();
        let blunder = Some(shared_types::Grade::Blunder);
        assert_eq!(grades, vec![blunder, None, blunder, None, blunder]);
    }

    #[test]
    fn a_winning_move_keeps_everything() {
        let mv = moves(&["wQ -bQ"]);
        // The game is over and Black, to move, has lost.
        let screen = vec![pos(90.0, "wA1 bQ/"), over(0.0)];
        let result = assemble(&mv, &screen, &unchecked(2), &SCHEME);
        let eval = result[0].as_ref().unwrap();
        assert_eq!(eval.after, 100.0);
        assert_eq!(eval.loss(), 0.0);
    }

    #[test]
    fn a_move_that_loses_the_game_on_the_spot_is_a_blunder() {
        let mv = moves(&["wB1 wQ/"]);
        let screen = vec![pos(70.0, "wA1 bQ/"), over(100.0)];
        assert_eq!(positions_to_recheck(&mv, &screen, &[], 2.0), vec![0, 1]);
        let result = assemble(&mv, &screen, &unchecked(2), &SCHEME);
        let eval = result[0].as_ref().unwrap();
        assert_eq!(eval.after, 0.0);
        assert_eq!(eval.grade(), Some(shared_types::Grade::Blunder));
    }

    #[test]
    fn a_drawing_move_keeps_half() {
        let mv = moves(&["wA1 bQ/"]);
        let screen = vec![pos(80.0, "wA1 bQ-"), over(50.0)];
        let result = assemble(&mv, &screen, &unchecked(2), &SCHEME);
        assert_eq!(result[0].as_ref().unwrap().after, 50.0);
    }

    #[test]
    fn a_move_that_leaves_only_a_pass_is_read_after_the_pass() {
        let mv = moves(&["wA1 bQ/", "pass", "wG1 -wL"]);
        // Black can only pass, and White, to move again, stands at 60.
        let screen = vec![
            pos(75.0, "wA1 bQ-"),
            Searched::OnlyPass,
            pos(60.0, "wG1 -wL"),
            pos(42.0, "x"),
        ];
        let result = assemble(&mv, &screen, &unchecked(4), &SCHEME);
        assert_eq!(result[0].as_ref().unwrap().after, 60.0);
        assert_eq!(result[1], None, "the pass itself is not judged");
        assert_eq!(positions_to_recheck(&mv, &screen, &[], 2.0), vec![0, 2]);
    }

    #[test]
    fn a_rechecked_move_across_a_pass_is_judged_at_full_sims() {
        let mv = moves(&["wA1 bQ/", "pass"]);
        let screen = vec![
            pos(75.0, "wA1 bQ-"),
            Searched::OnlyPass,
            pos(60.0, "wG1 -wL"),
        ];
        let full = vec![Some(pos(74.0, "wA1 bQ-")), None, Some(pos(58.0, "wG1 -wL"))];
        let result = assemble(&mv, &screen, &full, &SCHEME);
        let eval = result[0].as_ref().unwrap();
        assert_eq!((eval.before, eval.after, eval.sims), (74.0, 58.0, 800));
    }

    #[test]
    fn a_game_that_stops_on_a_forced_pass_leaves_the_move_unjudged() {
        let mv = moves(&["wA1 bQ/"]);
        let screen = vec![pos(75.0, "wA1 bQ-"), Searched::OnlyPass];
        assert_eq!(assemble(&mv, &screen, &unchecked(2), &SCHEME), vec![None]);
        assert!(positions_to_recheck(&mv, &screen, &[], 2.0).is_empty());
    }

    #[test]
    fn a_rechecked_move_is_judged_at_full_sims() {
        let mv = moves(&["wL", "bG1 wL/"]);
        let screen = vec![pos(65.0, "wP"), pos(45.0, "bP wL/"), pos(50.0, "wM /wL")];
        let full = vec![Some(pos(64.0, "wP")), Some(pos(48.0, "bP wL/")), None];
        let result = assemble(&mv, &screen, &full, &SCHEME);

        let first = result[0].as_ref().unwrap();
        assert_eq!((first.before, first.after, first.sims), (64.0, 52.0, 800));
        // The second move's after-position only has a screen search, so it counts as 50 sims,
        // but it starts from where the first move's full search left the game.
        let second = result[1].as_ref().unwrap();
        assert_eq!((second.before, second.after, second.sims), (48.0, 50.0, 50));
    }

    #[test]
    fn a_position_with_nothing_to_search_gives_no_judgement() {
        let mv = moves(&["pass"]);
        let screen = vec![Searched::OnlyPass, pos(50.0, "wA1 -bQ")];
        assert_eq!(assemble(&mv, &screen, &unchecked(2), &SCHEME), vec![None]);
    }

    fn with_line(winprob: f32, best: &str, line: Vec<String>) -> Searched {
        Searched::Position(PositionEval {
            winprob,
            best: best.to_string(),
            line,
        })
    }

    #[test]
    fn long_lines_are_cut_to_what_the_server_accepts() {
        let mv = moves(&["wL"]);
        let line = std::iter::once("wP".to_string())
            .chain((0..40).map(|i| format!("m{i}")))
            .collect();
        let screen = vec![with_line(65.0, "wP", line), pos(40.0, "x")];
        let result = assemble(&mv, &screen, &unchecked(2), &SCHEME);
        assert_eq!(result[0].as_ref().unwrap().line.len(), MAX_EVAL_LINE);
    }

    #[test]
    fn a_line_not_led_by_the_best_move_is_replaced_by_the_best_move() {
        let mv = moves(&["wL"]);
        let screen = vec![
            with_line(65.0, "wP", vec!["wG1".to_string()]),
            pos(40.0, "x"),
        ];
        let result = assemble(&mv, &screen, &unchecked(2), &SCHEME);
        assert_eq!(result[0].as_ref().unwrap().line, vec!["wP".to_string()]);
    }
}
