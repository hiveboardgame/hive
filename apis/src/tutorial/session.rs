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
    retried_attempts: Vec<u8>,
    exploration_moves: Vec<u8>,
    last_black_arrow: Option<(Position, Position)>,
    route_arrows: Vec<(Position, Position)>,
    revealed_pieces: Vec<Piece>,
}

impl Session {
    pub fn new(lesson: &'static Lesson) -> Result<Self, SessionError> {
        Self::with_starting_state(lesson, None)
    }

    pub fn with_starting_state(
        lesson: &'static Lesson,
        starting: Option<State>,
    ) -> Result<Self, SessionError> {
        let mut state = if let Some(state) = starting {
            state
        } else {
            match lesson.setup {
                Setup::ForcedPass { position, color } => {
                    let parsed = hop::parse(position)
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let mut state = State::new_from_position(parsed.board, parsed.game_type, color)
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    state
                        .play_turn_from_history("pass", "")
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    if state.turn_color != parsed.to_move {
                        return Err(SessionError::Position(
                            "forced pass leaves the wrong side to move".into(),
                        ));
                    }
                    state
                }
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
                Setup::BlankExercise {
                    position,
                    swap: None,
                    ..
                } => {
                    let position = hop::parse(position)
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    State::new_from_position(position.board, position.game_type, position.to_move)
                        .map_err(|error| SessionError::Position(error.to_string()))?
                }
                Setup::SwapPieces {
                    position,
                    first,
                    second,
                }
                | Setup::BlankExercise {
                    position,
                    swap: Some((first, second)),
                    ..
                } => {
                    let mut parsed = hop::parse(position)
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let first = first
                        .parse::<Piece>()
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let second = second
                        .parse::<Piece>()
                        .map_err(|error| SessionError::Position(error.to_string()))?;
                    let a = parsed.board.position_of_piece(first).ok_or_else(|| {
                        SessionError::Position("first swapped piece is missing".into())
                    })?;
                    let b = parsed.board.position_of_piece(second).ok_or_else(|| {
                        SessionError::Position("second swapped piece is missing".into())
                    })?;
                    let swap_piece = |piece| {
                        if piece == first {
                            second
                        } else if piece == second {
                            first
                        } else {
                            piece
                        }
                    };
                    let last_moved = parsed
                        .board
                        .last_moved
                        .map(|(piece, position)| (swap_piece(piece), position));
                    let last_move = parsed.board.last_move;
                    let stunned = parsed.board.stunned.map(swap_piece);
                    parsed.board.remove(a);
                    parsed.board.remove(b);
                    parsed.board.insert(a, second, false);
                    parsed.board.insert(b, first, false);
                    parsed.board.last_moved = last_moved;
                    parsed.board.last_move = last_move;
                    parsed.board.stunned = stunned;
                    State::new_from_position(parsed.board, parsed.game_type, parsed.to_move)
                        .map_err(|error| SessionError::Position(error.to_string()))?
                }
                Setup::Pending => return Err(SessionError::Pending),
                Setup::Guide(_) | Setup::TextOnly => State::new(GameType::MLP, false),
            }
        };
        if let Setup::BlankExercise {
            start_as, extras, ..
        } = lesson.setup
        {
            for (spec, target) in extras {
                let piece = spec
                    .parse::<Piece>()
                    .map_err(|error| SessionError::Position(error.to_string()))?;
                let at = hex(&state.board, target).ok_or_else(|| {
                    SessionError::Position(format!("extra tile target {target} is missing"))
                })?;
                if state.board.occupied(at) || state.board.position_of_piece(piece).is_some() {
                    return Err(SessionError::Position(
                        "extra tile overlaps an existing piece".into(),
                    ));
                }
                state.board.insert(at, piece, true);
            }
            if !extras.is_empty() {
                state = State::new_from_position(state.board, state.game_type, state.turn_color)
                    .map_err(|error| SessionError::Position(error.to_string()))?;
            }
            if start_as.is_some_and(|color| color != state.turn_color) {
                state.pass_out_of_turn();
            }
        }
        if let Some(from) = lesson
            .steps
            .first()
            .and_then(|step| step.previous_move_from)
        {
            let from = hex(&state.board, from)
                .ok_or_else(|| SessionError::Position("previous move origin is missing".into()))?;
            if state.board.occupied(from) {
                return Err(SessionError::Position(
                    "previous move origin must be empty".into(),
                ));
            }
            state.board.last_move = (Some(from), state.board.last_moved.map(|(_, at)| at));
        }
        let revealed_pieces = Self::neighbours_to_reveal(lesson, &state);
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
            retried_attempts: vec![0; lesson.steps.len()],
            exploration_moves: vec![0; lesson.steps.len()],
            last_black_arrow: None,
            route_arrows: Vec::new(),
            revealed_pieces,
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

    pub fn black_reply_arrow(&self) -> Option<(Position, Position)> {
        self.last_black_arrow
    }

    fn remember_black_move(&mut self, before: &State) {
        let Some((piece, to)) = self.state.board.last_moved else {
            return;
        };
        if piece.color() != Color::Black {
            return;
        }
        let Some(from) = before.board.position_of_piece(piece) else {
            self.last_black_arrow = None;
            return;
        };
        self.last_black_arrow = (from != to).then_some((from, to));
    }

    pub fn current_step(&self) -> Option<&'static Step> {
        self.lesson.steps.get(self.step)
    }

    pub fn hint(&self) -> Option<&'static str> {
        let step = self.current_step()?;
        Some(
            step.conditional_hint
                .filter(|(condition, _)| condition.holds(&self.state))
                .map_or(step.hint, |(_, hint)| hint),
        )
    }

    pub fn prompt(&self) -> Option<&'static str> {
        let step = self.current_step()?;
        if self.is_finished() {
            if let Some((color, prompt)) = step.win_prompt {
                if self.state.game_status
                    == GameStatus::Finished(hive_lib::GameResult::Winner(color))
                {
                    return Some(prompt);
                }
            }
        }
        Some(
            step.conditional_prompt
                .filter(|(condition, _)| condition.holds(&self.state))
                .map_or(step.prompt, |(_, prompt)| prompt),
        )
    }

    pub fn completion_result(&self) -> Option<&'static str> {
        if !self.is_finished() || !self.current_step()?.announce_winner {
            return None;
        }
        match self.state.game_status {
            GameStatus::Finished(hive_lib::GameResult::Winner(Color::White)) => Some("White wins!"),
            GameStatus::Finished(hive_lib::GameResult::Winner(Color::Black)) => Some("Black wins!"),
            _ => None,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.step >= self.lesson.steps.len()
            || self.at_closing_read()
            || self
                .current_step()
                .is_some_and(|step| step.complete_on_load)
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
            && self
                .lesson
                .blank_movers()
                .is_none_or(|movers| movers.iter().any(|spec| spec_matches(spec, piece)))
    }

    /// Blank tiles have no active abilities: only the visible piece's own movement is available.
    pub fn destinations(&self, piece: Piece) -> Vec<Position> {
        let normal_only = self.lesson.uses_blank_tiles()
            || self
                .current_step()
                .is_some_and(|step| step.normal_moves_only);
        Self::destinations_in(&self.state, self.learner, piece, normal_only)
    }

    fn destinations_in(state: &State, color: Color, piece: Piece, blank: bool) -> Vec<Position> {
        let Some(from) = state.board.position_of_piece(piece) else {
            return Vec::new();
        };
        let mut targets = state
            .board
            .moves(color)
            .get(&(piece, from))
            .cloned()
            .unwrap_or_default();
        if blank {
            let normal = hive_lib::Bug::normal_moves(from, &state.board);
            targets.retain(|target| normal.contains(target));
        }
        targets.sort_by_key(|position| (position.q, position.r));
        targets.dedup();
        targets
    }

    fn hint_visible(&self, step: &Step) -> bool {
        step.hint_after_retries
            .is_some_and(|limit| self.retried_attempts[self.step] >= limit)
            || step
                .hint_after
                .is_some_and(|limit| self.failures() >= limit)
            || step.hint_moves.is_some_and(|(_, limit)| {
                self.exploration_moves.get(self.step).copied().unwrap_or(0) >= limit
            })
    }

    fn failures(&self) -> u8 {
        self.failed_attempts.get(self.step).copied().unwrap_or(0)
    }

    fn annotation_board(&self) -> &hive_lib::Board {
        if self.needs_retry() {
            &self.checkpoint.board
        } else {
            &self.state.board
        }
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
                !step.immobile_marks
                    || !board
                        .moves(self.learner)
                        .iter()
                        .any(|((piece, _), targets)| {
                            spec_matches(step.marks[index], *piece) && !targets.is_empty()
                        })
            })
        })
    }

    fn neighbours_to_reveal(lesson: &Lesson, state: &State) -> Vec<Piece> {
        let Setup::BlankExercise {
            reveal_neighbours: Some(anchor),
            ..
        } = lesson.setup
        else {
            return Vec::new();
        };
        let Some(at) = hex(&state.board, anchor) else {
            return Vec::new();
        };
        at.positions_around()
            .filter_map(|at| state.board.top_piece(at))
            .collect()
    }

    fn remember_revealed(&mut self) {
        // This introductory exercise reveals only the movements available now.
        // Later Mosquito lessons intentionally retain previously revealed pieces.
        if self.lesson.id == "bugs.mosquito" {
            self.revealed_pieces = Self::neighbours_to_reveal(self.lesson, &self.state);
            return;
        }
        for piece in Self::neighbours_to_reveal(self.lesson, &self.state) {
            if !self.revealed_pieces.contains(&piece) {
                self.revealed_pieces.push(piece);
            }
        }
    }

    pub fn revealed_neighbours(&self) -> Vec<Piece> {
        self.revealed_pieces.clone()
    }

    pub fn timeout(&mut self, loser: Color) -> Outcome {
        if self.is_finished() || self.needs_retry() {
            return Outcome::Illegal;
        }
        let Some(step) = self.current_step() else {
            return Outcome::Illegal;
        };
        self.state.game_status =
            GameStatus::Finished(hive_lib::GameResult::Winner(loser.opposite_color()));
        if matches!(step.goal, Goal::WaitForTimeout(expected) if expected == loser) {
            self.complete_step(step)
        } else {
            self.failed_attempts[self.step] = self.failures().saturating_add(1);
            self.wrong = true;
            Outcome::Wrong
        }
    }

    pub fn green_marks(&self) -> Vec<Target> {
        self.current_step().map_or_else(Vec::new, |step| {
            self.targets(step.green_marks, false, |_| true)
        })
    }

    pub fn continue_label(&self) -> &'static str {
        self.current_step()
            .map_or("Continue", |step| step.continue_label)
    }

    pub fn red_marks(&self) -> Vec<Target> {
        self.current_step().map_or_else(Vec::new, |step| {
            let mut marks = self.targets(step.red_marks, false, |_| true);
            if let Some((condition, extra)) = step.conditional_red_marks {
                if condition.holds(&self.state) {
                    marks.extend(self.targets(extra, false, |_| true));
                }
            }
            marks
        })
    }

    pub fn placement_crosses(&self) -> Vec<Position> {
        let Some(step) = self.current_step() else {
            return Vec::new();
        };
        let board = self.annotation_board();
        let color = step.crosses_for.unwrap_or(self.learner);
        let anchors: Vec<_> = if step.crosses_for.is_some() {
            board
                .all_taken_positions()
                .filter(|at| {
                    board
                        .top_piece(*at)
                        .is_some_and(|piece| piece.color() == color)
                })
                .collect()
        } else {
            step.placement_crosses
                .and_then(|anchor| hex(board, anchor))
                .into_iter()
                .collect()
        };
        let mut crosses: Vec<_> = anchors
            .into_iter()
            .flat_map(|at| at.positions_around())
            .filter(|at| !board.occupied(*at) && !board.spawnable(color, *at))
            .collect();
        crosses.sort_by_key(|at| (at.q, at.r));
        crosses.dedup();
        crosses
    }

    pub fn highlights(&self) -> Vec<Position> {
        self.current_step().map_or_else(Vec::new, |step| {
            let hint_visible = self.hint_visible(step);
            step.highlights
                .iter()
                .chain(step.hint_highlights.iter().filter(|_| hint_visible))
                .filter_map(|notation| hex(self.annotation_board(), notation))
                .collect()
        })
    }

    /// Reserve room for later hints when loading a lesson, without revealing them or refitting after moves.
    pub fn fit_targets(&self) -> Vec<Position> {
        let mut targets: Vec<_> = self
            .lesson
            .steps
            .iter()
            .flat_map(|step| {
                step.highlights
                    .iter()
                    .chain(step.hint_highlights.iter())
                    .chain(step.forbidden.iter())
                    .chain(step.red_arrows.iter().map(|(_, to)| to))
                    .chain(step.hint_arrows.iter().flat_map(|(from, to)| [from, to]))
                    .chain(step.arrows.iter().map(|(_, to)| to))
            })
            .filter_map(|notation| hex(&self.initial.board, notation))
            .collect();
        targets.extend(self.initial.board.last_move.0);
        targets.extend(self.placement_crosses());
        for step in self.lesson.steps {
            if let Goal::LiftAnyOf { pieces, .. } = &step.goal {
                for at in pieces
                    .iter()
                    .filter_map(|piece| hex(&self.initial.board, piece))
                {
                    targets.extend(at.positions_around().flat_map(|at| at.positions_around()));
                }
            }
            if let Goal::LiftAny { piece, by } = &step.goal {
                if let (Ok(piece), Some(ability)) =
                    (piece.parse::<Piece>(), hex(&self.initial.board, by))
                {
                    if let Some(from) = self.initial.board.position_of_piece(piece) {
                        targets.extend(
                            Self::destinations_in(
                                &self.initial,
                                self.initial.turn_color,
                                piece,
                                false,
                            )
                            .into_iter()
                            .filter(|to| {
                                hive_lib::Bug::can_throw(ability, from, *to, &self.initial.board)
                            }),
                        );
                    }
                }
            }
            if let Some(piece) = step
                .legal_arrows
                .and_then(|spec| spec.parse::<Piece>().ok())
            {
                targets.extend(Self::destinations_in(
                    &self.initial,
                    self.initial.turn_color,
                    piece,
                    self.lesson.uses_blank_tiles() || step.normal_moves_only,
                ));
            }
        }
        // Reserve room for known replies before play starts. This keeps a new Black
        // piece and its arrow in view without fitting the board again after each move.
        let mut preview = self.clone();
        preview.retry();
        for _ in 0..self.lesson.steps.len() {
            let Some(Goal::Move { piece, to }) = preview.current_step().map(|step| &step.goal)
            else {
                break;
            };
            let (Ok(piece), Some(target)) = (piece.parse::<Piece>(), hex(&preview.state.board, to))
            else {
                break;
            };
            match preview.play(piece, target) {
                Outcome::StepDone | Outcome::LessonDone => {
                    if let Some((from, to)) = preview.black_reply_arrow() {
                        targets.extend([from, to]);
                    } else {
                        targets.extend(preview.state.board.last_move.1);
                    }
                }
                _ => break,
            }
            if preview.is_finished() {
                break;
            }
        }
        targets
    }

    pub fn forbidden(&self) -> Vec<Position> {
        self.current_step().map_or_else(Vec::new, |step| {
            step.forbidden
                .iter()
                .filter_map(|notation| hex(self.annotation_board(), notation))
                .filter(|position| !self.annotation_board().occupied(*position))
                .filter(|position| {
                    if let Goal::Travel { piece, .. } = step.goal {
                        piece
                            .parse::<Piece>()
                            .ok()
                            .is_none_or(|piece| !self.destinations(piece).contains(position))
                    } else {
                        true
                    }
                })
                .collect()
        })
    }

    pub fn arrows(&self) -> Vec<(Position, Position)> {
        if self.needs_retry() {
            return Vec::new();
        }
        self.current_step().map_or_else(Vec::new, |step| {
            let hint = self.hint_visible(step);
            let expand = |arrows: Vec<(Position, Position)>| -> Vec<(Position, Position)> {
                arrows
                    .into_iter()
                    .flat_map(|(from, to)| {
                        step.movement_paths.map_or_else(
                            || vec![(from, to)],
                            |bug| super::paths::movement_path(bug, &self.state.board, from, to),
                        )
                    })
                    .collect()
            };
            let mut arrows: Vec<_> =
                if hint && step.legal_arrows.is_some() && step.hint_highlights.is_empty() {
                    Vec::new()
                } else {
                    step.arrows
                        .iter()
                        .filter_map(|(from, to)| {
                            Some((
                                hex(self.annotation_board(), from)?,
                                hex(self.annotation_board(), to)?,
                            ))
                        })
                        .collect()
                };
            if !hint || !step.hint_highlights.is_empty() {
                if let Some(piece) = step
                    .legal_arrows
                    .and_then(|spec| spec.parse::<Piece>().ok())
                {
                    if let Some(from) = self.state.board.position_of_piece(piece) {
                        arrows.extend(expand(
                            self.destinations(piece)
                                .into_iter()
                                .map(|to| (from, to))
                                .collect(),
                        ));
                    }
                }
            }
            if hint {
                arrows.extend(step.hint_arrows.iter().filter_map(|(from, to)| {
                    Some((
                        hex(self.annotation_board(), from)?,
                        hex(self.annotation_board(), to)?,
                    ))
                }));
                if step.hint_highlights.is_empty() && step.hint_arrows.is_empty() {
                    arrows.extend(expand(self.move_hints(&step.goal)));
                }
            }
            if step.movement_paths.is_some() {
                // Show one complete three-step turn, updating after the learner moves.
                arrows.extend(expand(self.route_arrows.iter().take(1).copied().collect()));
            } else {
                arrows.extend(self.route_arrows.iter().copied());
            }
            let mut seen = std::collections::HashSet::new();
            arrows.retain(|edge| seen.insert(*edge));
            arrows
        })
    }

    pub fn red_arrows(&self) -> Vec<(Position, Position)> {
        if self.needs_retry() {
            return Vec::new();
        }
        self.current_step().map_or_else(Vec::new, |step| {
            if let Some(from) = step
                .backtrack_arrows
                .and_then(|piece| hex(&self.state.board, piece))
            {
                return super::paths::spider_backtracking_path(&self.state.board, from);
            }
            if step.initial_red_arrows && self.exploration_moves[self.step] > 0 {
                return Vec::new();
            }
            if step.legal_arrows.is_some() && self.hint_visible(step) {
                return Vec::new();
            }
            step.red_arrows
                .iter()
                .filter_map(|(piece, to)| {
                    let mover: Piece = piece.parse().ok()?;
                    let from = self.state.board.position_of_piece(mover)?;
                    let to = hex(&self.state.board, to)?;
                    let targets = if matches!(step.goal, Goal::Lift { .. }) {
                        hive_lib::Bug::normal_moves(from, &self.state.board)
                    } else {
                        self.destinations(mover)
                    };
                    (!targets.contains(&to)).then_some((from, to))
                })
                .collect()
        })
    }

    fn move_hints(&self, goal: &Goal) -> Vec<(Position, Position)> {
        if let Goal::Lift { piece, by, to } = goal {
            return self.lift_hints(piece, by, Some(to));
        }
        if let Goal::LiftAny { piece, by } = goal {
            return self.lift_hints(piece, by, None);
        }
        if let Goal::LiftAnyOf { pieces, by } = goal {
            return pieces
                .iter()
                .map(|piece| self.lift_hints(piece, by, None))
                .find(|hints| !hints.is_empty())
                .unwrap_or_default();
        }
        if let Goal::Explore { pieces, condition } = goal {
            return self.explore_hint(pieces, condition).into_iter().collect();
        }
        if let Goal::Travel { piece, to } = goal {
            let route = self.travel_route(piece, to);
            return if self
                .current_step()
                .is_some_and(|step| step.hint_moves.is_some())
            {
                route
            } else {
                route.into_iter().take(1).collect()
            };
        }
        if let Goal::MoveSatisfying {
            piece: spec,
            condition,
        } = goal
        {
            for ((piece, from), targets) in self.state.board.moves(self.learner) {
                if !spec_matches(spec, piece) {
                    continue;
                }
                for to in targets {
                    let mut next = self.state.clone();
                    if next.play_turn_from_position(piece, to).is_ok() && condition.holds(&next) {
                        return vec![(from, to)];
                    }
                }
            }
            return Vec::new();
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

    fn lift_hints(&self, spec: &str, by: &str, target: Option<&str>) -> Vec<(Position, Position)> {
        let board = &self.state.board;
        let (Ok(piece), Some(ability)) = (spec.parse::<Piece>(), hex(board, by)) else {
            return Vec::new();
        };
        let Some(from) = board.position_of_piece(piece) else {
            return Vec::new();
        };
        let required = target.and_then(|to| hex(board, to));
        self.destinations(piece)
            .into_iter()
            .find(|to| {
                required.is_none_or(|required| required == *to)
                    && hive_lib::Bug::can_throw(ability, from, *to, board)
            })
            .map_or_else(Vec::new, |to| vec![(from, ability), (ability, to)])
    }

    fn explore_hint(
        &self,
        specs: &[&str],
        condition: &super::goal::Condition,
    ) -> Option<(Position, Position)> {
        self.explore_route(specs, condition).first().copied()
    }

    /// Search only the movable pieces, with a fixed budget. Route hints are cached
    /// after a move rather than searching on every reactive board render.
    fn explore_route(
        &self,
        specs: &[&str],
        condition: &super::goal::Condition,
    ) -> Vec<(Position, Position)> {
        use std::collections::{HashSet, VecDeque};
        let pieces: Vec<Piece> = specs.iter().filter_map(|spec| spec.parse().ok()).collect();
        let key = |state: &State| {
            pieces
                .iter()
                .map(|piece| {
                    state
                        .board
                        .position_of_piece(*piece)
                        .map(|at| (at, state.board.level_of_piece(*piece, at)))
                })
                .collect::<Vec<_>>()
        };
        let mut seen = HashSet::from([key(&self.state)]);
        let mut queue = VecDeque::from([(self.state.clone(), Vec::new())]);
        while let Some((state, route)) = queue.pop_front() {
            let limit = self
                .current_step()
                .and_then(|step| step.retry_after_moves)
                .map_or(16, |limit| {
                    limit.saturating_sub(self.exploration_moves[self.step]) as usize
                });
            if route.len() >= limit {
                continue;
            }
            for piece in &pieces {
                let Some(from) = state.board.position_of_piece(*piece) else {
                    continue;
                };
                for to in Self::destinations_in(
                    &state,
                    self.learner,
                    *piece,
                    self.lesson.uses_blank_tiles(),
                ) {
                    if self
                        .current_step()
                        .and_then(|step| step.movement_paths)
                        .is_some_and(|bug| {
                            super::paths::movement_path(bug, &state.board, from, to).is_empty()
                        })
                    {
                        continue;
                    }
                    let mut next = state.clone();
                    next.hashes_count.clear();
                    if next.play_turn_from_position(*piece, to).is_err() {
                        continue;
                    }
                    let mut path = route.clone();
                    path.push((from, to));
                    if self
                        .current_step()
                        .is_some_and(|step| Self::tactical_reply_in(&next, step).is_some())
                    {
                        continue;
                    }
                    if condition.holds(&next) {
                        return path;
                    }
                    if matches!(next.game_status, GameStatus::Finished(_)) {
                        continue;
                    }
                    next.pass_out_of_turn();
                    if seen.insert(key(&next)) {
                        queue.push_back((next, path));
                    }
                    if seen.len() >= 2048 {
                        return Vec::new();
                    }
                }
            }
        }
        Vec::new()
    }

    fn travel_route(&self, spec: &str, notation: &str) -> Vec<(Position, Position)> {
        use std::collections::{HashSet, VecDeque};
        let Some(piece) = spec.parse::<Piece>().ok() else {
            return Vec::new();
        };
        let Some(target) = hex(&self.state.board, notation) else {
            return Vec::new();
        };
        let Some(start) = self.state.board.position_of_piece(piece) else {
            return Vec::new();
        };
        let mut visited = HashSet::from([start]);
        let mut queue = VecDeque::from([(self.state.clone(), Vec::new())]);
        while let Some((state, path)) = queue.pop_front() {
            for ((mover, from), targets) in state.board.moves(self.learner) {
                if mover != piece {
                    continue;
                }
                for to in targets {
                    if !visited.insert(to) {
                        continue;
                    }
                    let mut path = path.clone();
                    path.push((from, to));
                    if to == target {
                        return path;
                    }
                    let mut next = state.clone();
                    if next.play_turn_from_position(piece, to).is_ok() {
                        next.pass_out_of_turn();
                        queue.push_back((next, path));
                    }
                }
            }
            if visited.len() > 256 {
                break;
            }
        }
        Vec::new()
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
        if (self.lesson.uses_blank_tiles() || step.normal_moves_only)
            && self.state.board.position_of_piece(piece).is_some()
            && !self.destinations(piece).contains(&position)
        {
            return if self.forbidden().contains(&position) {
                self.wrong()
            } else {
                Outcome::Illegal
            };
        }
        // Repeated exploration is allowed in exercises without opponent turns.
        if matches!(step.goal, Goal::Explore { .. } | Goal::Travel { .. }) {
            self.state.hashes_count.clear();
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
        self.remember_black_move(&before);
        self.remember_revealed();
        if !matches!(step.goal, Goal::Explore { .. })
            && step.cover_reply.is_some()
            && self.refute_escape(step)
        {
            return Outcome::Refuted;
        }
        match &step.goal {
            Goal::Continue | Goal::WaitForTimeout(_) => {
                self.state = before;
                Outcome::Illegal
            }
            Goal::Explore { pieces, condition } => {
                if !pieces.iter().any(|spec| spec_matches(spec, piece))
                    || before.board.position_of_piece(piece).is_none()
                {
                    return self.wrong();
                }
                if self.refute_escape(step) {
                    return Outcome::Refuted;
                }
                if condition.holds(&self.state) {
                    self.complete_step(step)
                } else {
                    if matches!(self.state.game_status, GameStatus::Finished(_)) {
                        return self.wrong();
                    }
                    self.exploration_moves[self.step] =
                        self.exploration_moves[self.step].saturating_add(1);
                    if step
                        .retry_after_moves
                        .is_some_and(|limit| self.exploration_moves[self.step] >= limit)
                    {
                        return self.wrong();
                    }
                    self.hand_back();
                    self.checkpoint = self.state.clone();
                    if step
                        .route_hint_after
                        .is_some_and(|limit| self.exploration_moves[self.step] >= limit)
                    {
                        self.route_arrows = self.explore_route(pieces, condition);
                    }
                    Outcome::Progress
                }
            }
            Goal::Lift {
                piece: spec,
                by,
                to,
            } => {
                let lifted = spec_matches(spec, piece)
                    && hex(&before.board, to) == Some(position)
                    && hex(&before.board, by).is_some_and(|ability| {
                        before.board.position_of_piece(piece).is_some_and(|from| {
                            hive_lib::Bug::can_throw(ability, from, position, &before.board)
                        })
                    });
                if lifted {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::LiftAnyOf { pieces, by } => {
                let lifted = pieces.iter().any(|spec| spec_matches(spec, piece))
                    && hex(&before.board, by).is_some_and(|ability| {
                        before.board.position_of_piece(piece).is_some_and(|from| {
                            hive_lib::Bug::can_throw(ability, from, position, &before.board)
                        })
                    });
                if lifted {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::PlayAny => {
                if piece.is_color(self.learner) {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::LiftAny { piece: spec, by } => {
                let lifted = spec_matches(spec, piece)
                    && hex(&before.board, by).is_some_and(|ability| {
                        before.board.position_of_piece(piece).is_some_and(|from| {
                            hive_lib::Bug::can_throw(ability, from, position, &before.board)
                        })
                    });
                if lifted {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::MoveSatisfying {
                piece: spec,
                condition,
            } => {
                if spec_matches(spec, piece)
                    && before.board.position_of_piece(piece).is_some()
                    && condition.holds(&self.state)
                {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
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
                    self.exploration_moves[self.step] =
                        self.exploration_moves[self.step].saturating_add(1);
                    self.hand_back();
                    self.checkpoint = self.state.clone();
                    Outcome::Progress
                }
            }
            Goal::MovePiece {
                piece: spec,
                touching,
            } => {
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
            Goal::MoveExcept {
                piece: spec,
                except,
            } => {
                if spec_matches(spec, piece)
                    && before.board.position_of_piece(piece).is_some()
                    && hex(&before.board, except) != Some(position)
                {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::PlaceIn(hexes) => {
                if piece.is_color(self.learner)
                    && before.board.position_of_piece(piece).is_none()
                    && hexes
                        .iter()
                        .any(|to| hex(&before.board, to) == Some(position))
                {
                    self.complete_step(step)
                } else {
                    self.wrong()
                }
            }
            Goal::PlaceAny => {
                if piece.is_color(self.learner) && before.board.position_of_piece(piece).is_none() {
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
        if self.needs_retry() {
            self.retried_attempts[self.step] = self.retried_attempts[self.step].saturating_add(1);
        }
        self.state = self.initial.clone();
        self.revealed_pieces = Self::neighbours_to_reveal(self.lesson, &self.initial);
        self.checkpoint = self.initial.clone();
        self.step = 0;
        self.learner = self.initial.turn_color;
        self.exploration_moves.fill(0);
        self.last_black_arrow = None;
        self.route_arrows.clear();
        self.visited.clear();
        self.refuted = false;
        self.wrong = false;
    }

    fn wrong(&mut self) -> Outcome {
        self.failed_attempts[self.step] = self.failures().saturating_add(1);
        if let Some((condition, piece, to)) = self
            .current_step()
            .and_then(|step| step.conditional_refutation)
        {
            if condition.holds(&self.state) {
                let before = self.state.clone();
                let mut punished = before.clone();
                if punished.play_turn_from_history(piece, to).is_ok() {
                    self.state = punished;
                    self.remember_black_move(&before);
                    self.refuted = true;
                    return Outcome::Refuted;
                }
            }
        }
        if self
            .current_step()
            .is_some_and(|step| step.random_wrong_reply)
            && self.random_opponent_reply(None)
        {
            self.refuted = true;
            return Outcome::Refuted;
        }
        let refutations = self.current_step().map_or(&[][..], |step| step.refutations);
        for (piece, to) in refutations {
            let mut punished = self.state.clone();
            if punished.play_turn_from_history(piece, to).is_ok() {
                let before = self.state.clone();
                self.state = punished;
                self.remember_black_move(&before);
                self.refuted = true;
                return Outcome::Refuted;
            }
        }
        if let Some(piece) = self
            .current_step()
            .and_then(|step| step.wrong_reply)
            .and_then(|spec| spec.parse::<Piece>().ok())
        {
            for target in Self::destinations_in(&self.state, self.state.turn_color, piece, true) {
                let mut response = self.state.clone();
                if response.play_turn_from_position(piece, target).is_ok() {
                    let before = self.state.clone();
                    self.state = response;
                    self.remember_black_move(&before);
                    self.refuted = true;
                    return Outcome::Refuted;
                }
            }
        }
        self.wrong = true;
        Outcome::Wrong
    }

    fn complete_step(&mut self, step: &'static Step) -> Outcome {
        self.visited.clear();
        self.route_arrows.clear();
        self.step += 1;
        if self.step >= self.lesson.steps.len() {
            return Outcome::LessonDone;
        }
        match step.then {
            Reply::RandomPlacementNear { near } => {
                if self.state.turn_color == self.learner {
                    self.state.pass_out_of_turn();
                }
                if !self.random_placement_near(near) {
                    log::warn!(
                        "tutorial {}: no legal placement near {near}",
                        self.lesson.id
                    );
                }
            }
            Reply::InitialPosition => {
                self.state = self.initial.clone();
                self.learner = self.initial.turn_color;
                self.last_black_arrow = None;
            }
            Reply::RandomPreservingWin { piece, to } => {
                if !self.random_opponent_reply(Some((piece, to))) {
                    log::warn!("tutorial {}: no safe random reply", self.lesson.id);
                    self.hand_back();
                }
            }
            Reply::LearnerAgain => self.hand_back(),
            Reply::KeepTurn => {}
            Reply::NextPlayer(color) => {
                self.learner = color;
                self.hand_back();
            }
            Reply::Opponent { piece, to } => {
                if self.state.turn_color == self.learner {
                    self.state.pass_out_of_turn();
                }
                let before = self.state.clone();
                if let Err(error) = self.state.play_turn_from_history(piece, to) {
                    log::warn!(
                        "tutorial {}: scripted reply {piece} {to} failed: {error}",
                        self.lesson.id
                    );
                } else {
                    self.remember_black_move(&before);
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
                            .or_else(|| board.spawnable_positions(self.state.turn_color).next());
                        match target {
                            Some(target) => {
                                let before = self.state.clone();
                                if let Err(error) =
                                    self.state.play_turn_from_position(piece, target)
                                {
                                    log::warn!(
                                        "tutorial {}: scripted placement {piece} failed: {error}",
                                        self.lesson.id
                                    );
                                } else {
                                    self.remember_black_move(&before);
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
        self.remember_revealed();
        self.checkpoint = self.state.clone();
        if self.is_finished() {
            Outcome::LessonDone
        } else {
            Outcome::StepDone
        }
    }

    fn escape_reply_in(state: &State, spec: &str) -> Option<(Piece, Position)> {
        if matches!(state.game_status, GameStatus::Finished(_)) {
            return None;
        }
        let piece = spec.parse::<Piece>().ok()?;
        if piece.color() != state.turn_color {
            return None;
        }
        let mut targets = Self::destinations_in(state, state.turn_color, piece, false);
        targets.sort_by_key(|to| {
            (
                to.positions_around()
                    .filter(|at| state.board.occupied(*at))
                    .count(),
                to.q,
                to.r,
            )
        });
        targets
            .into_iter()
            .find(|to| {
                let mut reply = state.clone();
                reply.play_turn_from_position(piece, *to).is_ok()
            })
            .map(|to| (piece, to))
    }

    fn tactical_reply_in(state: &State, step: &Step) -> Option<(Piece, Position)> {
        if matches!(state.game_status, GameStatus::Finished(_)) {
            return None;
        }
        if let Some((beetles, targets)) = step.cover_reply {
            for spec in beetles {
                let Ok(beetle) = spec.parse::<Piece>() else {
                    continue;
                };
                if beetle.color() != state.turn_color {
                    continue;
                }
                for to in Self::destinations_in(state, state.turn_color, beetle, true) {
                    if !state
                        .board
                        .top_piece(to)
                        .is_some_and(|top| targets.iter().any(|target| spec_matches(target, top)))
                    {
                        continue;
                    }
                    let mut reply = state.clone();
                    if reply.play_turn_from_position(beetle, to).is_ok() {
                        return Some((beetle, to));
                    }
                }
            }
        }
        step.escape_reply
            .and_then(|spec| Self::escape_reply_in(state, spec))
    }

    fn refute_escape(&mut self, step: &Step) -> bool {
        let Some((piece, target)) = Self::tactical_reply_in(&self.state, step) else {
            return false;
        };
        let before = self.state.clone();
        if self.state.play_turn_from_position(piece, target).is_err() {
            return false;
        }
        self.remember_black_move(&before);
        self.failed_attempts[self.step] = self.failures().saturating_add(1);
        self.refuted = true;
        true
    }

    /// Sample only legal Black turns. On a successful exercise move, keep the
    /// intended next winning jump available; wrong attempts need no such filter.
    fn random_opponent_reply(&mut self, winning: Option<(&str, &str)>) -> bool {
        if matches!(self.state.game_status, GameStatus::Finished(_)) {
            return false;
        }
        let color = self.state.turn_color;
        let mut moves: Vec<_> = self
            .state
            .board
            .moves(color)
            .into_iter()
            .flat_map(|((piece, _), targets)| targets.into_iter().map(move |to| (piece, to)))
            .collect();
        let spawns: Vec<_> = self.state.board.spawnable_positions(color).collect();
        for piece in self
            .state
            .reserve(color)
            .values()
            .filter_map(|names| names.first())
            .filter_map(|name| name.parse::<Piece>().ok())
        {
            moves.extend(spawns.iter().map(|to| (piece, *to)));
        }
        moves.sort_by_key(|(piece, to)| (piece.to_string(), to.q, to.r));
        moves.dedup();
        let mut candidates = Vec::new();
        for (piece, to) in moves {
            let mut next = self.state.clone();
            if next.play_turn_from_position(piece, to).is_err()
                || matches!(next.game_status, GameStatus::Finished(_))
            {
                continue;
            }
            if let Some((piece, target)) = winning {
                let (Ok(piece), Some(target)) = (piece.parse::<Piece>(), hex(&next.board, target))
                else {
                    continue;
                };
                let mut finish = next.clone();
                if finish.play_turn_from_position(piece, target).is_err()
                    || finish.game_status
                        != GameStatus::Finished(hive_lib::GameResult::Winner(self.learner))
                {
                    continue;
                }
            }
            candidates.push((piece, to));
        }
        if candidates.is_empty() {
            return false;
        }
        let index = uuid::Uuid::new_v4().as_u128() as usize % candidates.len();
        let (piece, to) = candidates[index];
        let before = self.state.clone();
        if self.state.play_turn_from_position(piece, to).is_err() {
            return false;
        }
        self.remember_black_move(&before);
        true
    }

    fn random_placement_near(&mut self, near: &str) -> bool {
        let Some(anchor) = hex(&self.state.board, near) else {
            return false;
        };
        let color = self.state.turn_color;
        let spawns: Vec<_> = self
            .state
            .board
            .spawnable_positions(color)
            .filter(|at| at.is_neighbor(anchor))
            .collect();
        let mut candidates = Vec::new();
        for piece in self
            .state
            .reserve(color)
            .values()
            .filter_map(|pieces| pieces.first())
            .filter_map(|piece| piece.parse::<Piece>().ok())
        {
            for to in &spawns {
                let mut state = self.state.clone();
                if state.play_turn_from_position(piece, *to).is_ok() {
                    candidates.push((piece, *to));
                }
            }
        }
        if candidates.is_empty() {
            return false;
        }
        let index = uuid::Uuid::new_v4().as_u128() as usize % candidates.len();
        let (piece, to) = candidates[index];
        let before = self.state.clone();
        if self.state.play_turn_from_position(piece, to).is_err() {
            return false;
        }
        self.remember_black_move(&before);
        true
    }

    fn hand_back(&mut self) {
        if self.state.turn_color != self.learner {
            self.state.pass_out_of_turn();
        }
    }
}
