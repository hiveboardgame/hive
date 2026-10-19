use crate::{
    functions::game_evals::{evaluated_games, get_game_eval, EVALUATED_GAMES_PER_ASK},
    providers::{
        analysis::AnalysisContext,
        annotations::{AnnotationColor, Arrow},
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
    },
};
use hive_lib::{Board, Bug, Color, Direction, GameType, Piece, Position, State};
use leptos::{
    leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle},
    prelude::*,
    task::spawn_local,
};
use shared_types::{EvalResult, GameEvalView, GameId, Grade, MoveEval};
use std::{collections::HashSet, time::Duration};

const POLL_EVERY: Duration = Duration::from_secs(5);
const RETRY_AT_MOST_EVERY: Duration = Duration::from_secs(40);

/// Whether this viewer sees engine evals at all: admins only while the 2026 world championship
/// runs (the server enforces the same in `ensure_eval_access`). Return true to open them up.
pub fn evals_visible() -> Signal<bool> {
    let admin = expect_context::<AuthContext>().admin;
    // Account refreshes rebuild the auth state; only a real change may refetch evals.
    Memo::new(move |_| admin.get() == Some(true)).into()
}

#[derive(Clone, Copy)]
pub struct EvaluatedGames {
    pub ids: RwSignal<HashSet<GameId>>,
    /// Whether this viewer sees evals at all, and so whether rows keep room for a badge.
    pub shown: Signal<bool>,
}

/// Asks only about games it has not asked about yet: an infinite list grows a batch at a time.
pub fn provide_evaluated_games(games: Signal<Vec<GameId>>) {
    let visible = evals_visible();
    let evaluated = EvaluatedGames {
        ids: RwSignal::new(HashSet::new()),
        shown: visible,
    };
    provide_context(evaluated);
    let asked = StoredValue::new(HashSet::<GameId>::new());
    Effect::new(move |_| {
        if !visible.get() {
            return;
        }
        let fresh: Vec<GameId> = games.with(|games| {
            asked.with_value(|asked| {
                games
                    .iter()
                    .filter(|id| !asked.contains(*id))
                    .cloned()
                    .collect()
            })
        });
        asked.update_value(|asked| asked.extend(fresh.iter().cloned()));
        for chunk in fresh.chunks(EVALUATED_GAMES_PER_ASK) {
            let chunk = chunk.to_vec();
            spawn_local(async move {
                match evaluated_games(chunk.clone()).await {
                    Ok(found) => {
                        evaluated.ids.try_update(|ids| ids.extend(found));
                    }
                    Err(_) => {
                        asked.try_update_value(|asked| {
                            for id in &chunk {
                                asked.remove(id);
                            }
                        });
                    }
                }
            });
        }
    });
}

/// The engine eval of the game open in analysis. `None` until it has been fetched, and for
/// boards that are not a stored game.
#[derive(Clone, Copy)]
pub struct GameEvalContext {
    pub view: RwSignal<Option<GameEvalView>>,
    /// How many Evals panels are mounted. The board shows the eval's ghosts and arrows only
    /// while one is on screen; the History tab keeps a plain board.
    pub panels: RwSignal<u32>,
    /// What the Evals panel says about the picked move, and its step to the previous or next
    /// marked move: on a phone the panel sits a screen below the board, so the board repeats
    /// them.
    pub note: RwSignal<Option<String>>,
    pub step: RwSignal<Option<Callback<bool>>>,
    /// Leaves the engine's line, and where in it the board is: (step, steps).
    pub back: RwSignal<Option<Callback<()>>>,
    pub in_line: RwSignal<Option<(usize, usize)>>,
    /// The last fetch failed; it is retried, and what was shown before stays up meanwhile.
    pub unreachable: RwSignal<bool>,
    /// What the board draws for the picked move, worked out once for all of its layers.
    pub shown: Memo<Suggestion>,
}

pub fn waits_for_engine(view: Option<&GameEvalView>) -> bool {
    matches!(
        view,
        Some(GameEvalView::Queued { .. } | GameEvalView::Running { .. })
    )
}

impl GameEvalContext {
    pub fn new(analysis: AnalysisContext, game_state: GameStateStore) -> Self {
        let view = RwSignal::new(None);
        let panels = RwSignal::new(0);
        Self {
            view,
            panels,
            note: RwSignal::new(None),
            step: RwSignal::new(None),
            back: RwSignal::new(None),
            in_line: RwSignal::new(None),
            unreachable: RwSignal::new(false),
            shown: Memo::new(move |_| shown_on_board(analysis, game_state, view, panels)),
        }
    }

    pub fn follow(self, game_id: Memo<Option<GameId>>) {
        let refresh = RwSignal::new(0u32);
        let visible = evals_visible();
        let fetched = LocalResource::new(move || {
            let game_id = game_id.get().filter(|_| visible.get());
            refresh.track();
            async move {
                match game_id {
                    Some(game_id) => Some((game_id.clone(), get_game_eval(game_id).await)),
                    None => None,
                }
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
        let refresh_in = move |delay: Duration| {
            stop_polling();
            let fire = move || {
                poll.set_value(None);
                refresh.update(|n| *n += 1);
            };
            if let Ok(handle) = set_timeout_with_handle(fire, delay) {
                poll.set_value(Some(handle));
            }
        };
        let failures = StoredValue::new(0u32);
        Effect::watch(
            game_id,
            move |_, _, _| {
                self.view.set(None);
                self.unreachable.set(false);
                failures.set_value(0);
            },
            false,
        );
        Effect::new(move |_| match fetched.get() {
            // A slow answer about the game the board has just left.
            Some(Some((asked, _))) if game_id.get_untracked().as_ref() != Some(&asked) => {}
            Some(Some((_, Ok(view)))) => {
                failures.set_value(0);
                self.unreachable.set(false);
                if waits_for_engine(Some(&view)) {
                    refresh_in(POLL_EVERY);
                }
                // An unchanged eval must not rebuild the panel and lose the viewer's place.
                if self
                    .view
                    .with_untracked(|shown| shown.as_ref() != Some(&view))
                {
                    self.view.set(Some(view));
                }
            }
            // A dropped request (a deploy's slot flip, a flaky connection) keeps what is
            // shown and tries again, backing off while the server stays away.
            Some(Some((_, Err(_)))) => {
                let tries = failures.get_value() + 1;
                failures.set_value(tries);
                self.unreachable.set(true);
                refresh_in((POLL_EVERY * tries).min(RETRY_AT_MOST_EVERY));
            }
            Some(None) => {
                stop_polling();
                self.view.set(None);
            }
            None => {}
        });
        // A request or cancel from the panel starts or ends the polling itself.
        Effect::new(move |_| {
            if self.view.with(|view| waits_for_engine(view.as_ref())) {
                if poll.with_value(Option::is_none) {
                    refresh_in(POLL_EVERY);
                }
            } else if !self.unreachable.get_untracked() {
                stop_polling();
            }
        });
        on_cleanup(stop_polling);
    }

    /// The judgement of the game's move `ply` (0-based), once the eval is done.
    pub fn move_eval(&self, ply: usize) -> Option<MoveEval> {
        done_move_eval(self.view, ply)
    }

    pub fn grade(&self, ply: usize) -> Option<Grade> {
        self.move_eval(ply).and_then(|eval| eval.grade())
    }
}

fn done_move_eval(view: RwSignal<Option<GameEvalView>>, ply: usize) -> Option<MoveEval> {
    view.with(|view| match view {
        Some(GameEvalView::Done(result)) => result.moves.get(ply).cloned().flatten(),
        _ => None,
    })
}

/// Nothing while no Evals panel is on screen, or while a previewed line or explorer move
/// replaces the position the eval belongs to.
fn shown_on_board(
    analysis: AnalysisContext,
    game_state: GameStateStore,
    view: RwSignal<Option<GameEvalView>>,
    panels: RwSignal<u32>,
) -> Suggestion {
    if analysis.preview.with(Option::is_some) || panels.get() == 0 {
        return Suggestion::default();
    }
    analysis
        .store
        .selected_game_ply()
        .and_then(|ply| done_move_eval(view, ply))
        .map(|eval| {
            game_state
                .state()
                .with(|state| suggestion(&state.board, state.turn_color, &eval))
        })
        .unwrap_or_default()
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

/// What to show for a graded move: an arrow for a piece that moves, the piece on the cell it
/// ends on, so a placement from the reserve shows which piece goes there, and the Pillbug (or
/// Mosquito) that does the moving when a move throws a piece.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Suggestion {
    pub arrows: Vec<Arrow>,
    pub ghosts: Vec<Ghost>,
    pub throwers: Vec<(Position, AnnotationColor)>,
}

impl Suggestion {
    pub fn cells(&self) -> Vec<Position> {
        let mut cells: Vec<Position> = self.ghosts.iter().map(|ghost| ghost.position).collect();
        cells.extend(self.arrows.iter().flat_map(|arrow| [arrow.from, arrow.to]));
        cells.extend(self.throwers.iter().map(|(position, _)| *position));
        cells
    }
}

/// The mover's Pillbug, or a Mosquito copying one, next to `from`: the only way a move can
/// carry an opponent's piece.
fn thrower(board: &Board, from: Position, mover: Color) -> Option<(Piece, Position)> {
    [Bug::Pillbug, Bug::Mosquito].into_iter().find_map(|bug| {
        let piece = Piece::new_from(bug, mover, 0);
        let at = board.position_of_piece(piece)?;
        at.is_neighbor(from).then_some((piece, at))
    })
}

fn draw(shown: &mut Suggestion, board: &Board, mover: Color, uhp: &str, color: AnnotationColor) {
    if uhp == "pass" {
        return;
    }
    let (piece, target) = uhp.split_once(' ').unwrap_or((uhp, ""));
    let (Ok(piece), Ok(to)) = (piece.parse::<Piece>(), Position::from_string(target, board)) else {
        return;
    };
    if let Some(from) = board.position_of_piece(piece).filter(|from| *from != to) {
        shown.arrows.push(Arrow { from, to, color });
        if !piece.is_color(mover) {
            if let Some((_, at)) = thrower(board, from, mover) {
                shown.throwers.push((at, color));
            }
        }
    }
    shown.ghosts.push(Ghost {
        piece,
        position: to,
        level: board.level(to),
        color,
    });
}

/// What to draw on the position a graded move was played from, with `mover` to move: the move
/// played in red and the engine's better move in green.
pub fn suggestion(board: &Board, mover: Color, eval: &MoveEval) -> Suggestion {
    let mut shown = Suggestion::default();
    if eval.grade().is_some() {
        draw(&mut shown, board, mover, &eval.played, AnnotationColor::Red);
        draw(&mut shown, board, mover, &eval.best, AnnotationColor::Green);
    }
    shown
}

/// Where a move ends, the way the notation names it: beside a piece already on the board, or
/// on top of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spot {
    pub next_to: Piece,
    /// None for on top.
    pub side: Option<Direction>,
}

impl Spot {
    fn parse(target: &str) -> Option<Spot> {
        let (before, rest) = match target.chars().next()? {
            c @ ('\\' | '-' | '/') => (Some(c), &target[1..]),
            _ => (None, target),
        };
        let (name, after) = match rest.chars().last()? {
            c @ ('\\' | '-' | '/') => (&rest[..rest.len() - 1], Some(c)),
            _ => (rest, None),
        };
        let side = match (before, after) {
            (Some('\\'), _) => Some(Direction::NW),
            (Some('-'), _) => Some(Direction::W),
            (Some('/'), _) => Some(Direction::SW),
            (_, Some('/')) => Some(Direction::NE),
            (_, Some('-')) => Some(Direction::E),
            (_, Some('\\')) => Some(Direction::SE),
            _ => None,
        };
        Some(Spot {
            next_to: name.parse().ok()?,
            side,
        })
    }

    /// "left of White's Queen", "on top of Black's Beetle 1": always by colour, never "your",
    /// which a spectator reads as their own piece.
    fn words(&self) -> String {
        let side = match self.side {
            None => "on top of",
            Some(Direction::NW) => "above left of",
            Some(Direction::NE) => "above right of",
            Some(Direction::E) => "right of",
            Some(Direction::SE) => "below right of",
            Some(Direction::SW) => "below left of",
            Some(Direction::W) => "left of",
        };
        format!(
            "{side} {}'s {}",
            side_name(self.next_to.color()),
            bare_name(self.next_to)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Place {
        piece: Piece,
        at: Option<Spot>,
        cell: Position,
    },
    Move {
        piece: Piece,
        to: Option<Spot>,
        cell: Position,
    },
    Throw {
        thrower: Piece,
        piece: Piece,
        to: Option<Spot>,
        cell: Position,
    },
    Pass,
}

impl Action {
    fn cell(&self) -> Option<Position> {
        match self {
            Action::Place { cell, .. } | Action::Move { cell, .. } | Action::Throw { cell, .. } => {
                Some(*cell)
            }
            Action::Pass => None,
        }
    }
}

fn action(board: &Board, mover: Color, uhp: &str) -> Option<Action> {
    if uhp == "pass" {
        return Some(Action::Pass);
    }
    let (piece, target) = uhp.split_once(' ').unwrap_or((uhp, ""));
    let piece: Piece = piece.parse().ok()?;
    let cell = Position::from_string(target, board).ok()?;
    let spot = Spot::parse(target);
    Some(match board.position_of_piece(piece) {
        None => Action::Place {
            piece,
            at: spot,
            cell,
        },
        Some(from) if !piece.is_color(mover) => Action::Throw {
            thrower: thrower(board, from, mover)
                .map_or(Piece::new_from(Bug::Pillbug, mover, 0), |t| t.0),
            piece,
            to: spot,
            cell,
        },
        Some(_) => Action::Move {
            piece,
            to: spot,
            cell,
        },
    })
}

fn side_name(color: Color) -> &'static str {
    match color {
        Color::White => "White",
        Color::Black => "Black",
    }
}

/// "Ant 2", or "Queen" for a bug a player has just one of.
fn bare_name(piece: Piece) -> String {
    match piece.order() {
        0 => piece.bug().name().to_string(),
        n => format!("{} {n}", piece.bug().name()),
    }
}

/// "Ant 2", "the Queen", or for the opponent's pieces "Black's Ant 2".
fn piece_name(piece: Piece, mover: Color) -> String {
    match (piece.is_color(mover), piece.order()) {
        (true, 0) => format!("the {}", piece.bug().name()),
        (true, _) => bare_name(piece),
        (false, _) => format!("{}'s {}", side_name(piece.color()), bare_name(piece)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wording {
    pub mover: Color,
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

/// Verb forms: "placing"/"place".
struct Verbs {
    place: &'static str,
    move_: &'static str,
    use_: &'static str,
    pass: &'static str,
}

const DOING: Verbs = Verbs {
    place: "placing",
    move_: "moving",
    use_: "using",
    pass: "passing",
};
const DO: Verbs = Verbs {
    place: "place",
    move_: "move",
    use_: "use",
    pass: "pass",
};

impl Wording {
    fn spot(&self, spot: Option<Spot>) -> String {
        spot.map(|spot| format!(" {}", spot.words()))
            .unwrap_or_default()
    }

    /// The move in words. `instead` marks a better move that ends on the same cell as the
    /// played one, so only the piece differs.
    fn words(&self, action: Action, verbs: &Verbs, instead: bool) -> String {
        let place = |spot: Option<Spot>| {
            format!(
                "{}{}",
                self.spot(spot),
                if instead { " instead" } else { "" }
            )
        };
        match action {
            Action::Place { piece, at, .. } => {
                format!(
                    "{} {}{}",
                    verbs.place,
                    piece_name(piece, self.mover),
                    place(at)
                )
            }
            Action::Move { piece, to, .. } => {
                format!(
                    "{} {}{}",
                    verbs.move_,
                    piece_name(piece, self.mover),
                    place(to)
                )
            }
            Action::Throw {
                thrower, piece, to, ..
            } => format!(
                "{} {}{} to move {}{}",
                verbs.use_,
                piece_name(thrower, self.mover),
                if thrower.bug() == Bug::Mosquito {
                    " (copying a Pillbug)"
                } else {
                    ""
                },
                piece_name(piece, self.mover),
                place(to)
            ),
            Action::Pass => verbs.pass.to_string(),
        }
    }

    fn best_there(&self) -> bool {
        self.best.cell().is_some() && self.best.cell() == self.played.cell()
    }

    /// "Move 5, White: placing the Pillbug below left of White's Ladybug (red) was a mistake.
    /// Placing Beetle 1 below left of White's Ladybug instead (green) was better."
    pub fn headline(&self, move_number: usize, grade: Grade) -> String {
        let marked = |action: Action, words: String, color: &str| match action {
            Action::Pass => words,
            _ => format!("{words} ({color})"),
        };
        let what = match grade {
            Grade::Inaccuracy => "an inaccuracy",
            Grade::Mistake => "a mistake",
            Grade::Blunder => "a blunder",
        };
        format!(
            "Move {move_number}, {}: {} was {what}. {} was better.",
            side_name(self.mover),
            marked(self.played, self.words(self.played, &DOING, false), "red"),
            capitalised(marked(
                self.best,
                self.words(self.best, &DOING, self.best_there()),
                "green"
            )),
        )
    }

    /// "Pillbug placed left of White's Queen", "Beetle 1 climbed on top of Black's Ant 2", "Passed".
    pub fn played_short(&self) -> String {
        match self.played {
            Action::Place { piece, at, .. } => {
                format!("{} placed{}", bare_name(piece), self.spot(at))
            }
            Action::Move {
                piece,
                to: to @ Some(Spot { side: None, .. }),
                ..
            } => format!("{} climbed{}", bare_name(piece), self.spot(to)),
            Action::Move { piece, to, .. } => {
                format!("{} moved{}", bare_name(piece), self.spot(to))
            }
            Action::Throw {
                thrower, piece, to, ..
            } => format!(
                "{}{} moved {}{}",
                bare_name(thrower),
                if thrower.bug() == Bug::Mosquito {
                    " (copying a Pillbug)"
                } else {
                    ""
                },
                piece_name(piece, self.mover),
                self.spot(to)
            ),
            Action::Pass => "Passed".to_string(),
        }
    }

    /// What to do instead, after "Better: ".
    pub fn best_short(&self) -> String {
        self.words(self.best, &DO, self.best_there())
    }
}

/// The engine's line after the first `ply` moves of `history`, one "White: place Ant 1 left
/// of White's Queen" per move; it stops at a move that cannot be played.
pub fn line_words(
    game_type: GameType,
    tournament: bool,
    history: &[(String, String)],
    ply: usize,
    line: &[String],
) -> Vec<String> {
    let mut state = State::new(game_type, tournament);
    for (piece, position) in history.iter().take(ply) {
        if state.play_turn_from_history(piece, position).is_err() {
            return Vec::new();
        }
    }
    let mut out = Vec::with_capacity(line.len());
    for mv in line {
        let mover = state.turn_color;
        let Some(action) = action(&state.board, mover, mv) else {
            break;
        };
        let wording = Wording {
            mover,
            played: action,
            best: action,
        };
        let words = format!(
            "{}: {}",
            side_name(mover),
            wording.words(action, &DO, false)
        );
        // Only moves the board can show: the line player stops at the same one.
        let (piece, position) = mv.split_once(' ').unwrap_or((mv.as_str(), ""));
        if state.play_turn_from_history(piece, position).is_err() {
            break;
        }
        out.push(words);
    }
    out
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
        let mover = state.turn_color;
        let wording = result
            .moves
            .get(ply)
            .and_then(Option::as_ref)
            .filter(|eval| eval.grade().is_some())
            .and_then(|eval| {
                Some(Wording {
                    mover,
                    played: action(&state.board, mover, &eval.played)?,
                    best: action(&state.board, mover, &eval.best)?,
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

    fn state(history: &str) -> State {
        State::new_from_str(history, "Base+MLP").unwrap()
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

    fn shown(history: &str, eval: &MoveEval) -> (Board, Suggestion) {
        let s = state(history);
        let shown = suggestion(&s.board, s.turn_color, eval);
        (s.board, shown)
    }

    fn ghost(shown: &Suggestion, color: AnnotationColor) -> Ghost {
        *shown.ghosts.iter().find(|g| g.color == color).unwrap()
    }

    #[test]
    fn a_placement_shows_which_piece_goes_where() {
        let (b, shown) = shown("wL ;bL wL-;", &graded("wQ /wL", "wM /wL"));
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
        let (b, shown) = shown("wL ;bL wL-;wQ /wL;bQ bL/;", &graded("wQ wL\\", "wL bQ/"));
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
        assert!(shown.throwers.is_empty());
    }

    #[test]
    fn a_ghost_climbing_a_stack_sits_on_top_of_it() {
        let (_, shown) = shown("wL ;bL wL-;wQ /wL;bQ bL/;", &graded("wQ wL\\", "wQ wL"));
        assert_eq!(ghost(&shown, AnnotationColor::Green).level, 1);
    }

    /// Black's Ant 1 has walked up to White's Pillbug, so a white move of that ant is a throw.
    const THROW: &str = "wP ;bL wP-;wQ \\wP;bQ bL-;wA1 /wP;bA1 bQ-;wB1 -wQ;bA1 wP\\;";

    #[test]
    fn a_throw_marks_the_pillbug_doing_it() {
        let (b, shown) = shown(THROW, &graded("bA1 -wP", "wQ wP/"));
        let pillbug = b.position_of_piece("wP".parse().unwrap()).unwrap();
        assert_eq!(shown.throwers, vec![(pillbug, AnnotationColor::Red)]);
    }

    fn words(history: &str, played: &str, best: &str) -> Wording {
        let s = state(history);
        Wording {
            mover: s.turn_color,
            played: action(&s.board, s.turn_color, played).unwrap(),
            best: action(&s.board, s.turn_color, best).unwrap(),
        }
    }

    #[test]
    fn a_placement_reads_as_placing_the_piece_somewhere() {
        let w = words("wL ;bL wL-;", "wP wL/", r"wB1 wL\");
        assert_eq!(
            w.headline(3, Grade::Mistake),
            "Move 3, White: placing the Pillbug above right of White's Ladybug (red) was a mistake. Placing Beetle 1 below right of White's Ladybug (green) was better."
        );
        assert_eq!(
            (w.played_short(), w.best_short()),
            (
                "Pillbug placed above right of White's Ladybug".to_string(),
                "place Beetle 1 below right of White's Ladybug".to_string()
            )
        );
    }

    #[test]
    fn a_better_move_to_the_same_cell_reads_as_there_instead() {
        let w = words("wL ;bL wL-;", "wQ /wL", "wM /wL");
        assert_eq!(
            w.best_short(),
            "place the Mosquito below left of White's Ladybug instead"
        );
    }

    #[test]
    fn the_same_piece_moved_better_says_where_to() {
        let w = words("wL ;bL wL-;wQ /wL;bQ bL/;", r"wQ wL\", "wQ -wL");
        assert_eq!(
            w.headline(5, Grade::Blunder),
            "Move 5, White: moving the Queen below right of White's Ladybug (red) was a blunder. Moving the Queen left of White's Ladybug (green) was better."
        );
        assert_eq!(w.best_short(), "move the Queen left of White's Ladybug");
    }

    #[test]
    fn the_opponents_pieces_are_named_by_colour() {
        let w = words("wL ;bL wL-;wQ /wL;bQ bL/;", "wQ bL-", "wQ bQ");
        assert_eq!(w.played_short(), "Queen moved right of Black's Ladybug");
        assert_eq!(w.best_short(), "move the Queen on top of Black's Queen");
        assert_eq!(
            words("wL ;bL wL-;wQ /wL;bQ bL/;", "wQ bL-", "wQ wL").best_short(),
            "move the Queen on top of White's Ladybug"
        );
    }

    #[test]
    fn pieces_with_several_of_a_kind_are_named_by_number() {
        let w = words(
            r"wA1 ;bA1 wA1-;wQ /wA1;bQ bA1/;wA2 \wA1;bA2 bQ/;",
            "wA1 bA2/",
            "wA2 bQ-",
        );
        assert_eq!(w.played_short(), "Ant 1 moved above right of Black's Ant 2");
        assert_eq!(w.best_short(), "move Ant 2 right of Black's Queen");
    }

    #[test]
    fn a_pillbug_throw_says_whose_piece_moves_and_who_moves_it() {
        let w = words(THROW, "bA1 -wP", "wQ wP/");
        assert_eq!(
            w.headline(9, Grade::Blunder),
            "Move 9, White: using the Pillbug to move Black's Ant 1 left of White's Pillbug (red) was a blunder. Moving the Queen above right of White's Pillbug (green) was better."
        );
        assert_eq!(
            w.played_short(),
            "Pillbug moved Black's Ant 1 left of White's Pillbug"
        );
        assert_eq!(
            words(THROW, "wQ wP/", "bA1 -wP").best_short(),
            "use the Pillbug to move Black's Ant 1 left of White's Pillbug"
        );
    }

    #[test]
    fn a_pass_has_no_colour_to_point_at() {
        let w = words("wL ;bL wL-;wQ /wL;", "pass", "bB1 bL-");
        assert_eq!(
            w.headline(4, Grade::Inaccuracy),
            "Move 4, Black: passing was an inaccuracy. Placing Beetle 1 right of Black's Ladybug (green) was better."
        );
    }

    #[test]
    fn a_piece_moving_onto_another_climbs() {
        let w = words("wL ;bL wL-;wQ /wL;bQ bL/;", "wQ wL", "wQ -wL");
        assert_eq!(w.played_short(), "Queen climbed on top of White's Ladybug");
    }

    #[test]
    fn the_engines_line_reads_move_by_move_from_each_sides_view() {
        let history: Vec<(String, String)> =
            vec![("wL".into(), "".into()), ("bL".into(), "wL-".into())];
        let line = ["wQ /wL".to_string(), "bQ bL/".to_string()];
        assert_eq!(
            line_words(GameType::MLP, true, &history, 2, &line),
            vec![
                "White: place the Queen below left of White's Ladybug".to_string(),
                "Black: place the Queen above right of Black's Ladybug".to_string(),
            ]
        );
    }

    #[test]
    fn the_engines_line_stops_before_a_move_the_board_cannot_play() {
        let history: Vec<(String, String)> =
            vec![("wL".into(), "".into()), ("bL".into(), "wL-".into())];
        // White may not place next to Black's pieces only.
        let line = ["wQ /wL", "bQ bL/", "wA1 bQ/"].map(String::from);
        assert_eq!(line_words(GameType::MLP, true, &history, 2, &line).len(), 2);
    }

    #[test]
    fn the_notation_names_the_side_of_the_piece_a_move_ends_beside() {
        let spot = |target: &str| Spot::parse(target).map(|spot| spot.side);
        assert_eq!(spot(r"\wQ"), Some(Some(Direction::NW)));
        assert_eq!(spot("-wQ"), Some(Some(Direction::W)));
        assert_eq!(spot("/wQ"), Some(Some(Direction::SW)));
        assert_eq!(spot("wQ/"), Some(Some(Direction::NE)));
        assert_eq!(spot("wQ-"), Some(Some(Direction::E)));
        assert_eq!(spot(r"wQ\"), Some(Some(Direction::SE)));
        assert_eq!(spot("bB1"), Some(None));
        assert_eq!(spot(""), None);
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
        assert!(
            matches!(all[0].unwrap().played, Action::Place { piece, at: None, .. } if piece == "wL".parse().unwrap())
        );
        assert_eq!(all[0].unwrap().best_short(), "place the Pillbug instead");
        assert_eq!(all[1], None);
        assert_eq!(
            all[2].unwrap().best_short(),
            "place the Mosquito below left of White's Ladybug instead"
        );
    }

    #[test]
    fn an_ungraded_move_draws_nothing() {
        let mut fine = graded("wQ /wL", "wM /wL");
        fine.after = fine.before;
        assert_eq!(shown("wL ;bL wL-;", &fine).1, Suggestion::default());
    }

    #[test]
    fn a_played_pass_has_nothing_to_draw_but_the_better_move_still_shows() {
        let (_, shown) = shown("wL ;bL wL-;", &graded("pass", "wM /wL"));
        assert_eq!(shown.ghosts.len(), 1);
        assert_eq!(shown.ghosts[0].color, AnnotationColor::Green);
    }
}
