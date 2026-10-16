use super::{
    goal::{hex, spec_matches, Goal},
    lesson::{Lesson, Reply, Setup, Step},
};
use crate::hiveground::grid_positions;
use hive_lib::{hop, Color, GameStatus, GameType, Piece, Position, State};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("this lesson's board has not been set up yet")]
    Pending,
    #[error("the lesson position does not load: {0}")]
    Position(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Illegal,
    Wrong,
    Refuted,
    Visited,
    StepDone,
    Progress,
    LessonDone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub position: Position,
    pub number: Option<u8>,
}

#[derive(Clone)]
pub struct Session {
    lesson: &'static Lesson,
    step: usize,
    learner: Color,
    state: State,
    checkpoint: State,
    initial: State,
    visited: Vec<usize>,
    refuted: bool,
    wrong: bool,
    failed_attempts: Vec<u8>,
}

impl Session {
    pub fn new(lesson: &'static Lesson) -> Result<Self, SessionError> {
        Self::with_starting_state(lesson, None)
    }

    pub fn with_starting_state(
        lesson: &'static Lesson,
        starting: Option<State>,
    ) -> Result<Self, SessionError> {
        let state = if let Some(state) = starting {
            state
        } else {
            match lesson.setup {
                Setup::Clocked { moves, .. } => State::new_from_str(moves, "Base+MLP")
                    .map_err(|error| SessionError::Position(error.to_string()))?,
                Setup::Position(raw)
                | Setup::BlankPosition(raw)
                | Setup::AfterLesson { fallback: raw, .. } => {
                    let position = hop::parse(raw)
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    State::new_from_position(position.board, position.game_type, position.to_move)
                        .map_err(|error| SessionError::Position(error.to_string()))?
                }
                Setup::SwapPieces { position, first, second } => {
                    let mut parsed = hop::parse(position)
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let first = first.parse::<Piece>()
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let second = second.parse::<Piece>()
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let a = parsed.board.position_of_piece(first)
                        .ok_or_else(|| SessionError::Position("first swapped piece is missing".into()))?;
                    let b = parsed.board.position_of_piece(second)
                        .ok_or_else(|| SessionError::Position("second swapped piece is missing".into()))?;
                    parsed.board.remove(a);
                    parsed.board.remove(b);
                    parsed.board.insert(a, second, false);
                    parsed.board.insert(b, first, false);
                    State::new_from_position(parsed.board, parsed.game_type, parsed.to_move)
                        .map_err(|error| SessionError::Position(error.to_string()))?
                }
                Setup::Pending => return Err(SessionError::Pending),
                Setup::TextOnly => State::new(GameType::MLP, false),
            }
        };
        Ok(Self {
            lesson,
            step: 0,
            learner: state.turn_color,
            checkpoint: state.clone(),
            initial: state.clone(),
            state,
            visited: Vec::new(),
            refuted: false,
            wrong: false,
            failed_attempts: vec![0; lesson.steps.len()],
        })
    }

    pub fn lesson(&self) -> &'static Lesson {
        self.lesson
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn learner(&self) -> Color {
        self.learner
    }

    pub fn step_index(&self) -> usize {
        self.step
    }

    pub fn current_step(&self) -> Option<&'static Step> {
        self.lesson.steps.get(self.step)
    }

    pub fn is_finished(&self) -> bool {
        self.step >= self.lesson.steps.len() || self.at_closing_read()
    }

    /// A read step after the last move only comments on it, so making that move already finishes the lesson.
    fn at_closing_read(&self) -> bool {
        let steps = self.lesson.steps;
        self.step + 1 == steps.len()
            && matches!(steps[self.step].goal, Goal::Continue)
            && steps[..self.step]
                .iter()
                .any(|step| !matches!(step.goal, Goal::Continue))
    }

    pub fn awaits_continue(&self) -> bool {
        !self.is_finished()
            && self
                .current_step()
                .is_some_and(|step| matches!(step.goal, Goal::Continue))
    }

    pub fn accepts_moves(&self) -> bool {
        self.is_learners_turn() && !self.awaits_continue()
    }

    pub fn needs_retry(&self) -> bool {
        self.wrong || self.refuted
    }

    pub fn can_select(&self, piece: Piece) -> bool {
        self.is_learners_turn()
            && (!self.lesson.uses_blank_tiles() || piece.to_string() == "wQ")
    }

    fn failures(&self) -> u8 {
        self.failed_attempts.get(self.step).copied().unwrap_or(0)
    }

    fn annotation_board(&self) -> &hive_lib::Board {
        if self.needs_retry() { &self.checkpoint.board } else { &self.state.board }
    }

    pub fn is_refuted(&self) -> bool {
        self.refuted
    }

    pub fn is_learners_turn(&self) -> bool {
        !self.is_finished()
            && !self.needs_retry()
            && self.state.turn_color == self.learner
            && matches!(
                self.state.game_status,
                GameStatus::NotStarted | GameStatus::InProgress
            )
    }

    pub fn marks(&self) -> Vec<Target> {
        self.current_step().map_or_else(Vec::new, |step| {
            let board = self.annotation_board();
            self.targets(step.marks, step.number_marks, |index| {
                !step.immobile_marks || !board.moves(self.learner).iter().any(|((piece, _), targets)| {
                    spec_matches(step.marks[index], *piece) && !targets.is_empty()
                })
            })
        })
    }

    pub fn placement_crosses(&self) -> Vec<Position> {
        let Some(around) = self.current_step().and_then(|step| step.placement_crosses) else {
            return Vec::new();
        };
        let board = self.annotation_board();
        let Some(anchor) = hex(board, around) else { return Vec::new(); };
        anchor.positions_around()
            .filter(|position| !board.occupied(*position) && !board.spawnable(self.learner, *position))
            .collect()
    }

    pub fn highlights(&self) -> Vec<Position> {
        self.current_step().map_or_else(Vec::new, |step| {
            let hint_visible = step
                .hint_after
                .is_some_and(|limit| self.failures() >= limit);
            step.highlights
                .iter()
                .chain(step.hint_highlights.iter().filter(|_| hint_visible))
                .filter_map(|notation| hex(self.annotation_board(), notation))
                .collect()
        })
    }

    pub fn forbidden(&self) -> Vec<Position> {
        self.current_step().map_or_else(Vec::new, |step| {
            step.forbidden
                .iter()
                .filter_map(|notation| hex(self.annotation_board(), notation))
                .collect()
        })
    }

    pub fn arrows(&self) -> Vec<(Position, Position)> {
        if self.needs_retry() { return Vec::new(); }
        self.current_step().map_or_else(Vec::new, |step| {
            let mut arrows: Vec<_> = step
                .arrows
                .iter()
                .filter_map(|(from, to)| {
                    Some((hex(self.annotation_board(), from)?, hex(self.annotation_board(), to)?))
                })
                .collect();
            if step.hint_highlights.is_empty()
                && step.hint_after.is_some_and(|limit| self.failures() >= limit)
            {
                arrows.extend(self.move_hints(&step.goal));
            }
            arrows
        })
    }

    fn move_hints(&self, goal: &Goal) -> Vec<(Position, Position)> {
        if let Goal::Travel { piece, to } = goal {
            return self.travel_hint(piece, to).into_iter().collect();
        }
        let board = &self.state.board;
        let (spec, target, excluded, touching) = match goal {
            Goal::Move { piece, to } => (*piece, hex(board, to), None, None),
            Goal::MoveExcept { piece, except } => (*piece, None, hex(board, except), None),
            Goal::MovePiece { piece, touching } => (
                *piece,
                None,
                None,
                touching.and_then(|other| hex(board, other)),
            ),
            _ => return Vec::new(),
        };
        let mut hints: Vec<_> = board
            .moves(self.learner)
            .into_iter()
            .filter(|((piece, _), _)| spec_matches(spec, *piece))
            .flat_map(|((_, from), targets)| {
                targets
                    .into_iter()
                    .filter(move |position| {
                        target.is_none_or(|target| target == *position)
                            && excluded != Some(*position)
                            && touching.is_none_or(|other| position.is_neighbor(other))
                    })
                    .map(move |to| (from, to))
            })
            .collect();
        hints.dedup();
        if !matches!(goal, Goal::MovePiece { .. }) {
            hints.truncate(1);
        }
        hints
    }

    fn travel_hint(&self, spec: &str, notation: &str) -> Option<(Position, Position)> {
        use std::collections::{HashSet, VecDeque};
        let piece: Piece = spec.parse().ok()?;
        let target = hex(&self.state.board, notation)?;
        let start = self.state.board.position_of_piece(piece)?;
        let mut visited = HashSet::from([start]);
        let mut queue = VecDeque::from([(self.state.clone(), None)]);
        while let Some((state, first)) = queue.pop_front() {
            for ((mover, from), targets) in state.board.moves(self.learner) {
                if mover != piece { continue; }
                for to in targets {
                    if !visited.insert(to) { continue; }
                    let first = first.unwrap_or((from, to));
                    if to == target { return Some(first); }
                    let mut next = state.clone();
                    if next.play_turn_from_position(piece, to).is_ok() {
                        next.pass_out_of_turn();
                        queue.push_back((next, Some(first)));
                    }
                }
            }
            if visited.len() > 256 { break; }
        }
        None
    }

    pub fn stars(&self) -> Vec<Target> {
        let Some(Goal::Visit { hexes, .. }) = self.current_step().map(|step| &step.goal) else {
            return Vec::new();
        };
        self.targets(hexes, true, |index| !self.visited.contains(&index))
    }

    /// Optionally numbered by place in the lesson, so a hex keeps its number after others are visited.
    fn targets(
        &self,
        notations: &[&str],
        number_targets: bool,
        show: impl Fn(usize) -> bool,
    ) -> Vec<Target> {
        let numbered = number_targets && notations.len() > 1;
        notations
            .iter()
            .enumerate()
            .filter(|(index, _)| show(*index))
            .filter_map(|(index, notation)| {
                hex(self.annotation_board(), notation).map(|position| Target {
                    position,
                    number: numbered.then_some(index as u8 + 1),
                })
            })
            .collect()
    }

    /// The moves played so far in standard notation, for lessons that show them.
    pub fn written_moves(&self) -> Option<Vec<String>> {
        self.lesson
            .steps
            .iter()
            .any(|step| step.history)
            .then(|| self.played_moves())
    }

    pub fn played_moves(&self) -> Vec<String> {
        self.state
            .history
            .moves
            .iter()
            .filter(|(piece, _)| piece != "pass")
            .map(|(piece, position)| format!("{piece} {position}").trim().to_string())
            .collect()
    }

    pub fn grid(&self) -> Vec<Position> {
        if !self.current_step().is_some_and(|step| step.grid) {
            return Vec::new();
        }
        grid_positions(&self.state.board)
    }

    pub fn advance(&mut self) -> Outcome {
        match self.current_step() {
            Some(step) if matches!(step.goal, Goal::Continue) => self.complete_step(step),
            _ => Outcome::Illegal,
        }
    }

    pub fn play(&mut self, piece: Piece, position: Position) -> Outcome {
        let Some(step) = self.current_step() else {
            return Outcome::Illegal;
        };
        if !self.accepts_moves() || !self.can_select(piece) {
            return Outcome::Illegal;
        }
        let before = self.state.clone();
        if self.state.play_turn_from_position(piece, position).is_err() {
            self.state = before;
            return if self.forbidden().contains(&position) {
                self.wrong()
            } else {
                Outcome::Illegal
            };
        }
        match &step.goal {
            Goal::Continue => {
                self.state = before;
                Outcome::Illegal
            }
            Goal::Move { piece: spec, to } => {
                if spec_matches(spec, piece) && hex(&before.board, to) == Some(position) {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::Visit { piece: spec, hexes } => {
                if !spec_matches(spec, piece) {
                    return self.wrong();
                }
                if let Some(index) = hexes
                    .iter()
                    .position(|notation| hex(&before.board, notation) == Some(position))
                {
                    if !self.visited.contains(&index) {
                        self.visited.push(index);
                    }
                }
                if self.visited.len() == hexes.len() {
                    self.complete_step(step)
                } else {
                    self.state = self.checkpoint.clone();
                    Outcome::Visited
                }
            }
            Goal::Travel { piece: spec, to } => {
                if !spec_matches(spec, piece) || before.board.position_of_piece(piece).is_none() {
                    self.wrong()
                } else if hex(&before.board, to) == Some(position) {
                    self.complete_step(step)
                } else {
                    self.hand_back();
                    self.checkpoint = self.state.clone();
                    Outcome::Progress
                }
            }
            Goal::MovePiece { piece: spec, touching } => {
                let touches = touching.is_none_or(|other| {
                    hex(&before.board, other).is_some_and(|other| position.is_neighbor(other))
                });
                if spec_matches(spec, piece)
                    && before.board.position_of_piece(piece).is_some()
                    && touches
                {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::MoveExcept { piece: spec, except } => {
                if spec_matches(spec, piece)
                    && before.board.position_of_piece(piece).is_some()
                    && hex(&before.board, except) != Some(position)
                {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::MoveAny => {
                if piece.is_color(self.learner) && before.board.position_of_piece(piece).is_some() {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::Reach(condition) => {
                if condition.holds(&self.state) {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
        }
    }

    /// Retry the whole lesson; keep error counts so help survives repeated attempts.
    pub fn retry(&mut self) {
        self.state = self.initial.clone();
        self.checkpoint = self.initial.clone();
        self.step = 0;
        self.learner = self.initial.turn_color;
        self.visited.clear();
        self.refuted = false;
        self.wrong = false;
    }

    fn wrong(&mut self) -> Outcome {
        self.failed_attempts[self.step] = self.failures().saturating_add(1);
        let refutations = self.current_step().map_or(&[][..], |step| step.refutations);
        for (piece, to) in refutations {
            let mut punished = self.state.clone();
            if punished.play_turn_from_history(piece, to).is_ok() {
                self.state = punished;
                self.refuted = true;
                return Outcome::Refuted;
            }
        }
        self.wrong = true;
        Outcome::Wrong
    }

    fn complete_step(&mut self, step: &'static Step) -> Outcome {
        self.visited.clear();
        self.step += 1;
        if self.step >= self.lesson.steps.len() {
            return Outcome::LessonDone;
        }
        match step.then {
            Reply::LearnerAgain => self.hand_back(),
            Reply::NextPlayer(color) => {
                self.learner = color;
                self.hand_back();
            }
            Reply::Opponent { piece, to } => {
                if self.state.turn_color == self.learner {
                    self.state.pass_out_of_turn();
                }
                if let Err(error) = self.state.play_turn_from_history(piece, to) {
                    log::warn!(
                        "tutorial {}: scripted reply {piece} {to} failed: {error}",
                        self.lesson.id
                    );
                }
            }
            Reply::OpponentPlacement { piece, near } => {
                if self.state.turn_color == self.learner {
                    self.state.pass_out_of_turn();
                }
                match piece.parse::<Piece>() {
                    Ok(piece) => {
                        let board = &self.state.board;
                        let target = near
                            .and_then(|anchor| hex(board, anchor))
                            .and_then(|anchor| {
                                board
                                    .spawnable_positions(self.state.turn_color)
                                    .find(|position| position.is_neighbor(anchor))
                            })
                            .or_else(|| {
                                board.spawnable_positions(self.state.turn_color).next()
                            });
                        match target {
                            Some(target) => {
                                if let Err(error) =
                                    self.state.play_turn_from_position(piece, target)
                                {
                                    log::warn!(
                                        "tutorial {}: scripted placement {piece} failed: {error}",
                                        self.lesson.id
                                    );
                                }
                            }
                            None => log::warn!(
                                "tutorial {}: scripted placement {piece} has no legal target",
                                self.lesson.id
                            ),
                        }
                    }
                    Err(error) => log::warn!(
                        "tutorial {}: scripted placement {piece} does not parse: {error}",
                        self.lesson.id
                    ),
                }
            }
        }
        self.checkpoint = self.state.clone();
        if self.is_finished() {
            Outcome::LessonDone
        } else {
            Outcome::StepDone
        }
    }

    fn hand_back(&mut self) {
        if self.state.turn_color != self.learner {
            self.state.pass_out_of_turn();
        }
    }
}
