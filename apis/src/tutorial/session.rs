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
    visited: Vec<usize>,
    refuted: bool,
}

impl Session {
    pub fn new(lesson: &'static Lesson) -> Result<Self, SessionError> {
        let state = match lesson.setup {
            Setup::Clocked { moves, .. } => State::new_from_str(moves, "Base+MLP")
                .map_err(|error| SessionError::Position(error.to_string()))?,
            Setup::Position(raw) => {
                let position =
                    hop::parse(raw).map_err(|error| SessionError::Position(error.to_string()))?;
                State::new_from_position(position.board, position.game_type, position.to_move)
                    .map_err(|error| SessionError::Position(error.to_string()))?
            }
            Setup::Pending => return Err(SessionError::Pending),
            Setup::TextOnly => State::new(GameType::MLP, false),
        };
        Ok(Self {
            lesson,
            step: 0,
            learner: state.turn_color,
            checkpoint: state.clone(),
            state,
            visited: Vec::new(),
            refuted: false,
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

    pub fn is_refuted(&self) -> bool {
        self.refuted
    }

    pub fn is_learners_turn(&self) -> bool {
        !self.is_finished()
            && !self.refuted
            && self.state.turn_color == self.learner
            && matches!(
                self.state.game_status,
                GameStatus::NotStarted | GameStatus::InProgress
            )
    }

    pub fn marks(&self) -> Vec<Target> {
        self.current_step()
            .map_or_else(Vec::new, |step| self.targets(step.marks, |_| true))
    }

    pub fn highlights(&self) -> Vec<Position> {
        self.current_step().map_or_else(Vec::new, |step| {
            step.highlights
                .iter()
                .filter_map(|notation| hex(&self.state.board, notation))
                .collect()
        })
    }

    pub fn arrows(&self) -> Vec<(Position, Position)> {
        self.current_step().map_or_else(Vec::new, |step| {
            step.arrows
                .iter()
                .filter_map(|(from, to)| {
                    Some((hex(&self.state.board, from)?, hex(&self.state.board, to)?))
                })
                .collect()
        })
    }

    pub fn stars(&self) -> Vec<Target> {
        let Some(Goal::Visit { hexes, .. }) = self.current_step().map(|step| &step.goal) else {
            return Vec::new();
        };
        self.targets(hexes, |index| !self.visited.contains(&index))
    }

    /// Numbered by place in the lesson, so a hex keeps its number after others are visited.
    fn targets(&self, notations: &[&str], show: impl Fn(usize) -> bool) -> Vec<Target> {
        let numbered = notations.len() > 1;
        notations
            .iter()
            .enumerate()
            .filter(|(index, _)| show(*index))
            .filter_map(|(index, notation)| {
                hex(&self.state.board, notation).map(|position| Target {
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
        if !self.accepts_moves() {
            return Outcome::Illegal;
        }
        let before = self.state.clone();
        if self.state.play_turn_from_position(piece, position).is_err() {
            self.state = before;
            return Outcome::Illegal;
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

    pub fn retry(&mut self) {
        self.state = self.checkpoint.clone();
        self.visited.clear();
        self.refuted = false;
    }

    fn wrong(&mut self) -> Outcome {
        let refutations = self.current_step().map_or(&[][..], |step| step.refutations);
        for (piece, to) in refutations {
            let mut punished = self.state.clone();
            if punished.play_turn_from_history(piece, to).is_ok() {
                self.state = punished;
                self.refuted = true;
                return Outcome::Refuted;
            }
        }
        self.retry();
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
