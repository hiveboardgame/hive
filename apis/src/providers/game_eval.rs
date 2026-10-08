use crate::{
    functions::game_evals::get_game_eval,
    providers::{
        annotations::{AnnotationColor, Arrow},
        AuthContext,
    },
};
use hive_lib::{Board, GameType, Piece, Position, State};
use leptos::{
    leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle},
    prelude::*,
};
use shared_types::{EvalResult, GameEvalView, GameId, Grade, MoveEval};
use std::time::Duration;

/// How often an open game asks again while its eval waits or runs.
const POLL_EVERY: Duration = Duration::from_secs(5);

/// Whether this viewer sees engine evals at all: admins only while the 2026 world championship
/// runs (the server enforces the same in `ensure_eval_access`). Return true to open them up.
pub fn evals_visible() -> Signal<bool> {
    let admin = expect_context::<AuthContext>().admin;
    Signal::derive(move || admin.get() == Some(true))
}

/// The engine eval of the game open in analysis. `None` until it has been fetched, and for
/// boards that are not a stored game.
#[derive(Clone, Copy)]
pub struct GameEvalContext {
    pub view: RwSignal<Option<GameEvalView>>,
}

impl GameEvalContext {
    pub fn new() -> Self {
        Self {
            view: RwSignal::new(None),
        }
    }

    /// Keeps `view` in step with the eval of `game_id`, polling while it waits or runs.
    pub fn follow(self, game_id: Memo<Option<GameId>>) {
        let refresh = RwSignal::new(0u32);
        let visible = evals_visible();
        let fetched = LocalResource::new(move || {
            let game_id = game_id.get().filter(|_| visible.get());
            refresh.track();
            async move {
                match game_id {
                    Some(game_id) => get_game_eval(game_id).await.ok(),
                    None => None,
                }
            }
        });
        Effect::watch(game_id, move |_, _, _| self.view.set(None), false);
        Effect::new(move |_| {
            if let Some(view) = fetched.get() {
                self.view.set(view);
            }
        });
        let poll = StoredValue::new(None::<TimeoutHandle>);
        let stop_polling = move || {
            poll.update_value(|handle| {
                if let Some(handle) = handle.take() {
                    handle.clear();
                }
            })
        };
        Effect::new(move |_| {
            stop_polling();
            let waiting = self.view.with(|view| {
                matches!(
                    view,
                    Some(GameEvalView::Queued { .. } | GameEvalView::Running { .. })
                )
            });
            if waiting {
                if let Ok(handle) =
                    set_timeout_with_handle(move || refresh.update(|n| *n += 1), POLL_EVERY)
                {
                    poll.set_value(Some(handle));
                }
            }
        });
        on_cleanup(stop_polling);
    }

    /// The judgement of the game's move `ply` (0-based), once the eval is done.
    pub fn move_eval(&self, ply: usize) -> Option<MoveEval> {
        self.view.with(|view| match view {
            Some(GameEvalView::Done(result)) => result.moves.get(ply).cloned().flatten(),
            _ => None,
        })
    }

    pub fn grade(&self, ply: usize) -> Option<Grade> {
        self.move_eval(ply).and_then(|eval| eval.grade())
    }
}

impl Default for GameEvalContext {
    fn default() -> Self {
        Self::new()
    }
}

/// The piece a move puts on a cell, shown there on a plate of the move's colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ghost {
    pub piece: Piece,
    pub position: Position,
    /// Stack height the piece would sit at: on top of whatever is already there.
    pub level: usize,
    pub color: AnnotationColor,
}

/// What to show for a graded move: an arrow for a piece that moves, and the piece on the cell
/// it ends on, so a placement from the reserve shows which piece goes there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Suggestion {
    pub arrows: Vec<Arrow>,
    pub ghosts: Vec<Ghost>,
}

fn draw(shown: &mut Suggestion, board: &Board, uhp: &str, color: AnnotationColor) {
    if uhp == "pass" {
        return;
    }
    let (piece, target) = uhp.split_once(' ').unwrap_or((uhp, ""));
    let (Ok(piece), Ok(to)) = (piece.parse::<Piece>(), Position::from_string(target, board)) else {
        return;
    };
    if let Some(from) = board.position_of_piece(piece).filter(|from| *from != to) {
        shown.arrows.push(Arrow { from, to, color });
    }
    shown.ghosts.push(Ghost {
        piece,
        position: to,
        level: board.level(to),
        color,
    });
}

/// What to draw on the position a graded move was played from: the move played in red and
/// the engine's better move in green.
pub fn suggestion(board: &Board, eval: &MoveEval) -> Suggestion {
    let mut shown = Suggestion::default();
    if eval.grade().is_some() {
        draw(&mut shown, board, &eval.played, AnnotationColor::Red);
        draw(&mut shown, board, &eval.best, AnnotationColor::Green);
    }
    shown
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Place(Piece),
    Move(Piece),
    Pass,
}

fn action(board: &Board, uhp: &str) -> Option<Action> {
    if uhp == "pass" {
        return Some(Action::Pass);
    }
    let piece: Piece = uhp
        .split_once(' ')
        .map_or(uhp, |(piece, _)| piece)
        .parse()
        .ok()?;
    Some(match board.position_of_piece(piece) {
        Some(_) => Action::Move(piece),
        None => Action::Place(piece),
    })
}

/// "Ant 2" for a bug a player has several of, "the Queen" for one they have just one of.
fn piece_name(piece: Piece) -> String {
    match piece.order() {
        0 => format!("the {}", piece.bug().name()),
        n => format!("{} {n}", piece.bug().name()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wording {
    pub played: Action,
    pub best: Action,
}

fn capitalised(text: String) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

impl Wording {
    fn played_doing(&self) -> String {
        match self.played {
            Action::Place(piece) => format!("placing {}", piece_name(piece)),
            Action::Move(piece) => format!("moving {}", piece_name(piece)),
            Action::Pass => "passing".to_string(),
        }
    }

    /// The engine's move as a verb phrase: "placing Beetle 1" (`doing`) or "place Beetle 1"
    /// (after "Better: ").
    fn best_words(&self, doing: bool) -> String {
        let (place, move_, pass) = if doing {
            ("placing", "moving", "passing")
        } else {
            ("place", "move", "pass")
        };
        match (self.played, self.best) {
            (Action::Move(played), Action::Move(best)) if played == best => {
                format!("{move_} it elsewhere")
            }
            (_, Action::Place(piece)) => format!("{place} {}", piece_name(piece)),
            (_, Action::Move(piece)) => format!("{move_} {}", piece_name(piece)),
            (_, Action::Pass) => pass.to_string(),
        }
    }

    /// "Move 5, White: placing the Pillbug (red) was a mistake. Placing Beetle 1 (green) was
    /// better."
    pub fn headline(&self, move_number: usize, white: bool, grade: Grade) -> String {
        let marked = |action: Action, doing: String, color: &str| match action {
            Action::Pass => doing,
            _ => format!("{doing} ({color})"),
        };
        let what = match grade {
            Grade::Inaccuracy => "an inaccuracy",
            Grade::Mistake => "a mistake",
            Grade::Blunder => "a blunder",
        };
        format!(
            "Move {move_number}, {}: {} was {what}. {} was better.",
            if white { "White" } else { "Black" },
            marked(self.played, self.played_doing(), "red"),
            capitalised(marked(self.best, self.best_words(true), "green")),
        )
    }

    /// "Pillbug placed", "Ant 2 moved", "Passed".
    pub fn played_short(&self) -> String {
        let name =
            |piece: Piece| capitalised(piece_name(piece).trim_start_matches("the ").to_string());
        match self.played {
            Action::Place(piece) => format!("{} placed", name(piece)),
            Action::Move(piece) => format!("{} moved", name(piece)),
            Action::Pass => "Passed".to_string(),
        }
    }

    /// What to do instead, after "Better: ".
    pub fn best_short(&self) -> String {
        self.best_words(false)
    }
}

/// Puts every graded move of the game into words. Whether a piece is placed or moved depends
/// on the position, so the game is replayed once rather than per move.
pub fn wordings(
    game_type: GameType,
    tournament: bool,
    history: &[(String, String)],
    result: &EvalResult,
) -> Vec<Option<Wording>> {
    let mut state = State::new(game_type, tournament);
    let mut out = Vec::with_capacity(history.len());
    for (ply, (piece, position)) in history.iter().enumerate() {
        let wording = result
            .moves
            .get(ply)
            .and_then(Option::as_ref)
            .filter(|eval| eval.grade().is_some())
            .and_then(|eval| {
                Some(Wording {
                    played: action(&state.board, &eval.played)?,
                    best: action(&state.board, &eval.best)?,
                })
            });
        out.push(wording);
        if state.play_turn_from_history(piece, position).is_err() {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_lib::State;

    fn board(history: &str) -> Board {
        State::new_from_str(history, "Base+MLP").unwrap().board
    }

    fn graded(played: &str, best: &str) -> MoveEval {
        MoveEval {
            played: played.to_string(),
            best: best.to_string(),
            line: vec![best.to_string()],
            before: 60.0,
            after: 40.0,
            sims: 800,
        }
    }

    fn ghost(shown: &Suggestion, color: AnnotationColor) -> Ghost {
        *shown.ghosts.iter().find(|g| g.color == color).unwrap()
    }

    #[test]
    fn a_placement_shows_which_piece_goes_where() {
        let b = board("wL ;bL wL-;");
        let shown = suggestion(&b, &graded("wQ /wL", "wM /wL"));
        assert!(shown.arrows.is_empty());
        assert_eq!(
            ghost(&shown, AnnotationColor::Red).position,
            Position::from_string("/wL", &b).unwrap()
        );
        assert_eq!(
            ghost(&shown, AnnotationColor::Red).piece,
            "wQ".parse().unwrap()
        );
        assert_eq!(
            ghost(&shown, AnnotationColor::Green).piece,
            "wM".parse().unwrap()
        );
    }

    #[test]
    fn a_piece_on_the_board_gets_an_arrow_and_a_ghost_where_it_lands() {
        let b = board("wL ;bL wL-;wQ /wL;bQ bL/;");
        let shown = suggestion(&b, &graded("wQ wL\\", "wL bQ/"));
        let lady = b.position_of_piece("wL".parse().unwrap()).unwrap();
        let to = Position::from_string("bQ/", &b).unwrap();
        let green = shown
            .arrows
            .iter()
            .find(|a| a.color == AnnotationColor::Green)
            .unwrap();
        assert_eq!((green.from, green.to), (lady, to));
        assert_eq!(ghost(&shown, AnnotationColor::Green).position, to);
        assert_eq!(shown.arrows.len(), 2);
    }

    #[test]
    fn a_ghost_climbing_a_stack_sits_on_top_of_it() {
        let b = board("wL ;bL wL-;wQ /wL;bQ bL/;");
        let shown = suggestion(&b, &graded("wQ wL\\", "wQ wL"));
        assert_eq!(ghost(&shown, AnnotationColor::Green).level, 1);
    }

    fn words(history: &str, played: &str, best: &str) -> Wording {
        let b = board(history);
        Wording {
            played: action(&b, played).unwrap(),
            best: action(&b, best).unwrap(),
        }
    }

    #[test]
    fn a_placement_reads_as_placing_the_piece() {
        let w = words("wL ;bL wL-;", "wP wL/", r"wB1 wL\");
        assert_eq!(
            w.headline(3, true, Grade::Mistake),
            "Move 3, White: placing the Pillbug (red) was a mistake. Placing Beetle 1 (green) was better."
        );
        assert_eq!(
            (w.played_short(), w.best_short()),
            ("Pillbug placed".to_string(), "place Beetle 1".to_string())
        );
    }

    #[test]
    fn moving_the_same_piece_better_reads_as_elsewhere() {
        let w = words("wL ;bL wL-;wQ /wL;bQ bL/;", r"wQ wL\", "wQ -wL");
        assert_eq!(
            w.headline(5, true, Grade::Blunder),
            "Move 5, White: moving the Queen (red) was a blunder. Moving it elsewhere (green) was better."
        );
        assert_eq!(w.best_short(), "move it elsewhere");
    }

    #[test]
    fn pieces_with_several_of_a_kind_are_named_by_number() {
        let w = words(
            r"wA1 ;bA1 wA1-;wQ /wA1;bQ bA1/;wA2 \wA1;bA2 bQ/;",
            "wA1 bA2/",
            "wA2 bQ-",
        );
        assert_eq!(w.played_short(), "Ant 1 moved");
        assert_eq!(w.best_short(), "move Ant 2");
    }

    #[test]
    fn a_pass_has_no_colour_to_point_at() {
        let w = Wording {
            played: Action::Pass,
            best: Action::Place("bB1".parse().unwrap()),
        };
        assert_eq!(
            w.headline(9, false, Grade::Inaccuracy),
            "Move 9, Black: passing was an inaccuracy. Placing Beetle 1 (green) was better."
        );
    }

    #[test]
    fn wordings_follow_the_position_each_move_was_played_in() {
        let history: Vec<(String, String)> = vec![
            ("wL".into(), "".into()),
            ("bL".into(), "wL-".into()),
            ("wQ".into(), "/wL".into()),
        ];
        let mut result = EvalResult {
            moves: vec![None, None, Some(graded("wQ /wL", "wM /wL"))],
        };
        // Move 1 places the Ladybug; graded, it must read as a placement, not a move.
        result.moves[0] = Some(graded("wL", "wP"));
        let all = wordings(GameType::MLP, true, &history, &result);
        assert_eq!(all[0].unwrap().played, Action::Place("wL".parse().unwrap()));
        assert_eq!(all[1], None);
        assert_eq!(all[2].unwrap().best, Action::Place("wM".parse().unwrap()));
    }

    #[test]
    fn an_ungraded_move_draws_nothing() {
        let b = board("wL ;bL wL-;");
        let mut fine = graded("wQ /wL", "wM /wL");
        fine.after = fine.before;
        assert_eq!(suggestion(&b, &fine), Suggestion::default());
    }

    #[test]
    fn a_played_pass_has_nothing_to_draw_but_the_better_move_still_shows() {
        let b = board("wL ;bL wL-;");
        let shown = suggestion(&b, &graded("pass", "wM /wL"));
        assert_eq!(shown.ghosts.len(), 1);
        assert_eq!(shown.ghosts[0].color, AnnotationColor::Green);
    }
}
