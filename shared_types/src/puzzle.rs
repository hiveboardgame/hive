use hive_lib::{hop, Color, GameResult, GameStatus, GameType, Piece, Position, State};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

pub const PUZZLE_START_RATING: f64 = 1500.0;
pub const PUZZLE_START_DEVIATION: f64 = 150.0;
pub const PUZZLE_START_VOLATILITY: f64 = 0.06;
/// Hidden automatically once this many players voted and fewer than `AUTO_HIDE_MAX_APPROVAL` liked it.
pub const AUTO_HIDE_MIN_VOTES: i32 = 20;
pub const AUTO_HIDE_MAX_APPROVAL: f64 = 0.3;
/// The pseudo-collection of puzzles mined from the viewer's own games.
pub const MY_GAMES_COLLECTION: &str = "my-games";
/// The pseudo-collection of earlier puzzles of the day.
pub const OLD_DAILIES_COLLECTION: &str = "old-dailies";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PuzzleKind {
    /// Force a Queen surround within the stated number of moves.
    WinIn,
}

impl fmt::Display for PuzzleKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PuzzleKind::WinIn => write!(f, "WinIn"),
        }
    }
}

impl FromStr for PuzzleKind {
    type Err = PuzzleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "WinIn" => Ok(PuzzleKind::WinIn),
            other => Err(PuzzleError::UnknownKind(other.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PuzzleGameTypeFilter {
    #[default]
    Any,
    Mlp,
    Base,
}

impl PuzzleGameTypeFilter {
    pub fn game_type(self) -> Option<&'static str> {
        match self {
            PuzzleGameTypeFilter::Any => None,
            PuzzleGameTypeFilter::Mlp => Some("Base+MLP"),
            PuzzleGameTypeFilter::Base => Some("Base"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PuzzleDifficulty {
    Easier,
    #[default]
    Normal,
    Harder,
}

impl PuzzleDifficulty {
    pub fn rating_offset(self) -> f64 {
        match self {
            PuzzleDifficulty::Easier => -300.0,
            PuzzleDifficulty::Normal => 0.0,
            PuzzleDifficulty::Harder => 300.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PuzzleLength {
    #[default]
    Any,
    One,
    Two,
    ThreePlus,
}

impl PuzzleLength {
    /// The solver-move range, inclusive; `None` as the upper end means "or longer".
    pub fn solver_moves(self) -> Option<(i32, Option<i32>)> {
        match self {
            PuzzleLength::Any => None,
            PuzzleLength::One => Some((1, Some(1))),
            PuzzleLength::Two => Some((2, Some(2))),
            PuzzleLength::ThreePlus => Some((3, None)),
        }
    }
}

/// Which puzzles the trainer draws from. `theme` and `collection` come from the browse page.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(default)]
pub struct PuzzleSelection {
    pub game_type: PuzzleGameTypeFilter,
    pub difficulty: PuzzleDifficulty,
    pub length: PuzzleLength,
    pub theme: Option<String>,
    pub collection: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PuzzleHistoryFilter {
    #[default]
    All,
    Solved,
    Failed,
    /// Puzzles the player has not voted on yet.
    Unvoted,
}

/// One puzzle as the generator writes it and the import endpoint receives it (one JSON per line).
/// The start is either a game `history` (the site's `piece position;` string, whose last move is
/// the opponent's setup move) or a HOP `position`. `solution` is the main line; `variations` are
/// further complete lines that open with another, equally fast, solver move.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PuzzleImport {
    pub kind: PuzzleKind,
    pub game_type: String,
    #[serde(default)]
    pub history: Option<String>,
    #[serde(default)]
    pub position: Option<String>,
    pub solution: Vec<String>,
    #[serde(default)]
    pub variations: Vec<Vec<String>>,
    #[serde(default)]
    pub themes: Vec<String>,
    pub authors: Vec<String>,
    pub source: String,
    #[serde(default)]
    pub source_game: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
}

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PuzzleError {
    #[error("unknown puzzle kind {0}")]
    UnknownKind(String),
    #[error("a puzzle needs exactly one of history and position")]
    StartAmbiguous,
    #[error("the start does not load: {0}")]
    BadStart(String),
    #[error("the game is already over at the start")]
    StartFinished,
    #[error("the solution is empty")]
    EmptySolution,
    #[error("a puzzle must credit at least one author")]
    NoAuthors,
    #[error("a puzzle must name its source")]
    NoSource,
    #[error("line {line}, move {index} ({token}) is illegal: {reason}")]
    IllegalMove {
        line: usize,
        index: usize,
        token: String,
        reason: String,
    },
    #[error("line {line} ended before move {index}")]
    EndedEarly { line: usize, index: usize },
    #[error("line {0} does not end in a win for the solver")]
    NotAWin(usize),
    #[error("line {0} is not as long as the main line")]
    UnequalLength(usize),
    #[error("lines {0} and {1} answer the same solver moves differently")]
    ConflictingReplies(usize, usize),
}

/// Splits `piece position` notation; a lone piece (the first move of a game) has no position.
pub fn split_move(token: &str) -> (&str, &str) {
    match token.trim().split_once(' ') {
        Some((piece, position)) => (piece, position.trim()),
        None => (token.trim(), ""),
    }
}

/// The board cell and piece a notation token names on `state`, or None for `pass`.
pub fn resolve_move(state: &State, token: &str) -> Result<Option<(Piece, Position)>, String> {
    let (piece, position) = split_move(token);
    if piece == "pass" {
        return Ok(None);
    }
    let piece = Piece::from_str(piece).map_err(|e| e.to_string())?;
    let target = if position.is_empty() {
        Position::initial_spawn_position()
    } else if !position.contains(['-', '/', '\\']) {
        let under = Piece::from_str(position).map_err(|e| e.to_string())?;
        state
            .board
            .position_of_piece(under)
            .ok_or_else(|| format!("{under} is not on the board"))?
    } else {
        Position::from_string(position, &state.board).map_err(|e| e.to_string())?
    };
    Ok(Some((piece, target)))
}

/// Plays one solution token the way the site plays a move: a shut-out opponent auto-passes, and
/// an explicit `pass` that was already auto-played is consumed.
pub fn play_token(state: &mut State, token: &str) -> Result<(), String> {
    match resolve_move(state, token)? {
        None => {
            if state.history.last_move_is_pass() {
                return Ok(());
            }
            state
                .play_turn_from_history("pass", "")
                .map_err(|e| e.to_string())
        }
        Some((piece, position)) => state
            .play_turn_from_position(piece, position)
            .map_err(|e| e.to_string()),
    }
}

impl PuzzleImport {
    pub fn start_state(&self) -> Result<State, PuzzleError> {
        start_state(
            &self.game_type,
            self.history.as_deref(),
            self.position.as_deref(),
        )
    }

    pub fn lines(&self) -> Vec<Vec<String>> {
        std::iter::once(self.solution.clone())
            .chain(self.variations.iter().cloned())
            .collect()
    }

    /// Everything the import endpoint refuses to store: missing credits, or a line that is illegal,
    /// does not win, or disagrees with another about the defence.
    pub fn verify(&self) -> Result<Color, PuzzleError> {
        if self.authors.iter().all(|author| author.trim().is_empty()) {
            return Err(PuzzleError::NoAuthors);
        }
        if self.source.trim().is_empty() {
            return Err(PuzzleError::NoSource);
        }
        if self.solution.is_empty() {
            return Err(PuzzleError::EmptySolution);
        }
        verify_lines(&self.start_state()?, &self.lines())
    }
}

pub fn start_state(
    game_type: &str,
    history: Option<&str>,
    position: Option<&str>,
) -> Result<State, PuzzleError> {
    let state = match (history, position) {
        (Some(history), None) => State::new_from_str(history, game_type)
            .map_err(|e| PuzzleError::BadStart(e.to_string()))?,
        (None, Some(position)) => {
            let parsed = hop::parse(position).map_err(|e| PuzzleError::BadStart(e.to_string()))?;
            let expected =
                GameType::from_str(game_type).map_err(|e| PuzzleError::BadStart(e.to_string()))?;
            if parsed.game_type != expected {
                return Err(PuzzleError::BadStart(format!(
                    "position is {} but the puzzle says {expected}",
                    parsed.game_type
                )));
            }
            State::new_from_position(parsed.board, parsed.game_type, parsed.to_move)
                .map_err(|e| PuzzleError::BadStart(e.to_string()))?
        }
        _ => return Err(PuzzleError::StartAmbiguous),
    };
    if matches!(
        state.game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    ) {
        return Err(PuzzleError::StartFinished);
    }
    Ok(state)
}

/// Replays every line from `start` and returns the solver's colour if they all win, have the same
/// length, and answer identical solver moves identically.
pub fn verify_lines(start: &State, lines: &[Vec<String>]) -> Result<Color, PuzzleError> {
    let solver = start.turn_color;
    let Some(main) = lines.first() else {
        return Err(PuzzleError::EmptySolution);
    };
    if main.is_empty() {
        return Err(PuzzleError::EmptySolution);
    }
    let mut boards: Vec<Vec<Option<(Piece, Position)>>> = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        if line.len() != main.len() {
            return Err(PuzzleError::UnequalLength(line_index));
        }
        let mut state = start.clone();
        let mut resolved = Vec::new();
        for (index, token) in line.iter().enumerate() {
            if matches!(state.game_status, GameStatus::Finished(_)) {
                return Err(PuzzleError::EndedEarly {
                    line: line_index,
                    index,
                });
            }
            let illegal = |reason: String| PuzzleError::IllegalMove {
                line: line_index,
                index,
                token: token.clone(),
                reason,
            };
            resolved.push(resolve_move(&state, token).map_err(illegal)?);
            play_token(&mut state, token).map_err(illegal)?;
        }
        if state.game_status != GameStatus::Finished(GameResult::Winner(solver)) {
            return Err(PuzzleError::NotAWin(line_index));
        }
        boards.push(resolved);
    }
    for (a, first) in boards.iter().enumerate() {
        for (b, second) in boards.iter().enumerate().skip(a + 1) {
            let shared = first.iter().zip(second).take_while(|(x, y)| x == y).count();
            // Diverging on a defender move (odd index) means one solver line, two answers.
            if shared < first.len() && shared % 2 == 1 {
                return Err(PuzzleError::ConflictingReplies(a, b));
            }
        }
    }
    Ok(solver)
}

/// Canonical hash of a puzzle's start, shared with the opening explorer's position hashes.
pub fn start_hash(state: &State) -> Result<i64, PuzzleError> {
    let hop = hop::from_position(&state.board, state.game_type, state.turn_color);
    hop::to_hash(&hop).map_err(|e| PuzzleError::BadStart(e.to_string()))
}

/// Solver moves in a solution: every other token, starting with the first.
pub fn solver_moves(solution: &[String]) -> usize {
    solution.len().div_ceil(2)
}

/// Where a solver is inside a puzzle: which lines still agree with the moves played so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuzzleProgress {
    pub index: usize,
    pub open_lines: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PuzzleVerdict {
    /// Correct; the defender answers with this token (from the current state, after the move).
    Continue {
        reply: String,
    },
    /// Correct, and it ends the puzzle.
    Solved,
    Wrong,
}

impl PuzzleProgress {
    pub fn new(lines: &[Vec<String>]) -> Self {
        Self {
            index: 0,
            open_lines: (0..lines.len()).collect(),
        }
    }

    /// Judges a solver move against the lines still open and advances past the defender reply.
    /// `before` is the position the move is played from. At the last solver move any surround
    /// wins, so alternative finishes need no listing.
    pub fn judge(
        &mut self,
        lines: &[Vec<String>],
        before: &State,
        piece: Piece,
        target: Position,
    ) -> PuzzleVerdict {
        let Some(main) = lines.first() else {
            return PuzzleVerdict::Wrong;
        };
        let solver = before.turn_color;
        let is_last = self.index + 1 >= main.len();
        if is_last {
            let mut after = before.clone();
            let wins = after.play_turn_from_position(piece, target).is_ok()
                && after.game_status == GameStatus::Finished(GameResult::Winner(solver));
            return if wins {
                self.index += 1;
                PuzzleVerdict::Solved
            } else {
                PuzzleVerdict::Wrong
            };
        }
        let matching: Vec<usize> = self
            .open_lines
            .iter()
            .copied()
            .filter(|line| {
                lines[*line]
                    .get(self.index)
                    .is_some_and(|token| resolve_move(before, token) == Ok(Some((piece, target))))
            })
            .collect();
        let Some(&first) = matching.first() else {
            return PuzzleVerdict::Wrong;
        };
        let reply = lines[first]
            .get(self.index + 1)
            .cloned()
            .unwrap_or_default();
        self.open_lines = matching;
        self.index += 2;
        PuzzleVerdict::Continue { reply }
    }

    /// The token the solver should play next, from the first open line (for hints and reveal).
    pub fn expected<'a>(&self, lines: &'a [Vec<String>]) -> Option<&'a String> {
        self.open_lines
            .first()
            .and_then(|line| lines.get(*line))
            .and_then(|line| line.get(self.index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mined from game bCP1Vzx29s: a win in two with a single line.
    const HISTORY: &str = include_str!("../test_data/puzzle_history.txt");

    fn puzzle() -> PuzzleImport {
        serde_json::from_str(include_str!("../test_data/puzzle.json")).expect("fixture puzzle")
    }

    #[test]
    fn the_fixture_verifies() {
        let puzzle = puzzle();
        assert_eq!(puzzle.history.as_deref(), Some(HISTORY.trim()));
        assert!(puzzle.verify().is_ok(), "{:?}", puzzle.verify());
    }

    #[test]
    fn refuses_puzzles_without_credits() {
        let mut puzzle = puzzle();
        puzzle.authors = vec![" ".to_string()];
        assert_eq!(puzzle.verify(), Err(PuzzleError::NoAuthors));
        let mut puzzle = self::puzzle();
        puzzle.source = String::new();
        assert_eq!(puzzle.verify(), Err(PuzzleError::NoSource));
    }

    #[test]
    fn refuses_a_line_that_does_not_win() {
        let mut puzzle = puzzle();
        puzzle.solution.pop();
        assert!(puzzle.verify().is_err());
    }

    #[test]
    fn judges_the_main_line_through_to_solved() {
        let puzzle = puzzle();
        let lines = puzzle.lines();
        let mut state = puzzle.start_state().expect("start");
        let mut progress = PuzzleProgress::new(&lines);
        loop {
            let token = progress.expected(&lines).expect("a move to play").clone();
            let (piece, target) = resolve_move(&state, &token)
                .expect("resolves")
                .expect("not a pass");
            match progress.judge(&lines, &state, piece, target) {
                PuzzleVerdict::Continue { reply } => {
                    play_token(&mut state, &token).expect("solver move plays");
                    play_token(&mut state, &reply).expect("reply plays");
                }
                PuzzleVerdict::Solved => break,
                PuzzleVerdict::Wrong => panic!("the main line was judged wrong"),
            }
        }
    }

    #[test]
    fn a_direction_beside_a_piece_is_not_a_stack_on_it() {
        let state = puzzle().start_state().expect("start");
        let piece = state
            .board
            .all_taken_positions()
            .find_map(|at| state.board.top_piece(at))
            .expect("a piece on the board");
        let beside = resolve_move(&state, &format!("wA1 {piece}\\"))
            .expect("resolves")
            .expect("not a pass")
            .1;
        let on_top = resolve_move(&state, &format!("wA1 {piece}"))
            .expect("resolves")
            .expect("not a pass")
            .1;
        assert_ne!(beside, on_top);
        assert_eq!(Some(on_top), state.board.position_of_piece(piece));
    }

    #[test]
    fn split_move_handles_the_first_move() {
        assert_eq!(split_move("wG1"), ("wG1", ""));
        assert_eq!(split_move("wQ -bA1"), ("wQ", "-bA1"));
    }
}
