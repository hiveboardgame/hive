use crate::{
    common::{CurrentConfirm, MoveConfirm, PieceType},
    functions::puzzles::{next_puzzle, puzzle_defence, puzzle_refutation, submit_puzzle_attempt},
    hiveground::{
        HivegroundAction,
        HivegroundActions,
        HivegroundCapabilities,
        HivegroundInteraction,
    },
    providers::{
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
        AuthIdentity,
        Config,
    },
    responses::{PuzzleAttemptResponse, PuzzleResponse},
};
use hive_lib::{Bug, Color, GameStatus, Piece, Position, State};
use leptos::{leptos_dom::helpers::set_timeout, prelude::*, task::spawn_local};
use shared_types::{
    play_token,
    resolve_move,
    split_move,
    start_state,
    PuzzleDifficulty,
    PuzzleGameTypeFilter,
    PuzzleLength,
    PuzzleProgress,
    PuzzleVerdict,
};
use std::time::Duration;

const SETUP_DELAY: Duration = Duration::from_millis(600);
const REPLY_DELAY: Duration = Duration::from_millis(1300);
const SOLUTION_STEP: Duration = Duration::from_millis(2400);
const NOTICE_DURATION: Duration = Duration::from_millis(3500);
const GUEST_SEEN_KEY: &str = "puzzles.seen";
const GUEST_SEEN_LIMIT: usize = 200;
const GAME_TYPE_KEY: &str = "puzzles.game_type";
const ONE_CLICK_KEY: &str = "puzzles.one_click";
const DIFFICULTY_KEY: &str = "puzzles.difficulty";
const LENGTH_KEY: &str = "puzzles.length";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuzzlePhase {
    Loading,
    /// No puzzle could be found (an empty pool for the filter, or a bad link).
    Empty,
    /// The opponent's last move is being shown.
    Setup,
    Solving,
    /// A correct move was played; the opponent is answering.
    Replying,
    Solved,
    Failed,
    /// The solution is being (or has been) played out.
    Revealed,
}

impl PuzzlePhase {
    pub fn is_over(self) -> bool {
        matches!(
            self,
            PuzzlePhase::Solved | PuzzlePhase::Failed | PuzzlePhase::Revealed
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DailyStatus {
    pub tried: i64,
    pub solved: i64,
    pub my_result: Option<bool>,
    /// Days in a row the viewer solved the daily before today's, for signed-in viewers.
    pub streak: Option<i64>,
}

#[derive(Clone, Copy)]
pub struct PuzzleContext {
    pub game_state: GameStateStore,
    pub puzzle: RwSignal<Option<PuzzleResponse>>,
    pub phase: RwSignal<PuzzlePhase>,
    pub progress: RwSignal<PuzzleProgress>,
    /// The position the puzzle starts from, after the setup move.
    start: StoredValue<Option<State>>,
    pub hinted: RwSignal<bool>,
    pub hint_text: RwSignal<Option<String>>,
    /// 0 = no hint yet, 1 = the piece was named, 2 = the destination was shown too.
    pub hint_level: RwSignal<u8>,
    pub hint_target: RwSignal<Option<Position>>,
    pub hint_piece: RwSignal<Option<Piece>>,
    /// The opponent answered the wrong move with a defence that ends the forced win.
    pub refuted: RwSignal<bool>,
    /// Where the wrong move landed, so its mark stays put after the opponent answers.
    pub wrong_at: RwSignal<Option<Position>>,
    pub pending_notice: StoredValue<Option<String>>,
    /// Whether the solver has played a move since this puzzle loaded (skipping is then gone).
    pub moved: RwSignal<bool>,
    /// Set by Replay after a result: the board is practice and nothing is rated.
    pub replaying: RwSignal<bool>,
    /// A one-off explanation, e.g. why a selected piece shows no moves.
    pub notice: RwSignal<Option<String>>,
    /// Set once this load's first outcome went to the server; later tries are practice.
    pub reported: RwSignal<bool>,
    pub attempt: RwSignal<Option<PuzzleAttemptResponse>>,
    pub game_type: RwSignal<PuzzleGameTypeFilter>,
    pub difficulty: RwSignal<PuzzleDifficulty>,
    pub length: RwSignal<PuzzleLength>,
    /// A theme or collection picked on the browse page; training then draws only from it.
    pub theme: RwSignal<Option<String>>,
    pub collection: RwSignal<Option<String>>,
    /// The solution being shown, and how many of its moves are on the board.
    pub reveal_tokens: RwSignal<Vec<String>>,
    /// The board before the solution and after each of its moves, for stepping through it.
    reveal_states: StoredValue<Vec<State>>,
    pub reveal_step: RwSignal<usize>,
    /// Bumped on every load so delayed moves from an earlier puzzle never land on this one.
    generation: StoredValue<u64>,
    started_at: StoredValue<f64>,
    pub solved_streak: RwSignal<u32>,
    /// The opponent's latest move in words (the setup move, then each reply).
    pub opponent_move: RwSignal<Option<String>>,
    pub setup_note: StoredValue<Option<String>>,
    /// First Show solution press (button or S) on a rated try only arms it; the second plays it.
    pub reveal_armed: RwSignal<bool>,
    pub first_result: RwSignal<Option<bool>>,
    /// Whether the solver has reached the win at least once on this load (for replay wording).
    pub ever_solved: RwSignal<bool>,
    /// Whether the puzzle had been solved before the current practice replay started.
    pub solved_before_replay: RwSignal<bool>,
    /// Per-browser choice to play puzzle moves with a single click.
    pub one_click: RwSignal<bool>,
    /// Training follows the player's rating and rewrites the address bar per puzzle.
    pub training: StoredValue<bool>,
    /// Today's tally on the puzzle-of-the-day page.
    pub daily: RwSignal<Option<DailyStatus>>,
    /// After a miss the board stays open so the player can see how the position plays out.
    pub playing_out: RwSignal<bool>,
    /// Solver moves played since the miss.
    pub played_out: RwSignal<u32>,
    /// A difficulty for the next pick only, e.g. a harder puzzle after solving the daily.
    pub one_off_difficulty: StoredValue<Option<PuzzleDifficulty>>,
}

impl PuzzleContext {
    pub fn new(game_state: GameStateStore) -> Self {
        Self {
            game_state,
            puzzle: RwSignal::new(None),
            phase: RwSignal::new(PuzzlePhase::Loading),
            progress: RwSignal::new(PuzzleProgress::new(&[])),
            start: StoredValue::new(None),
            hinted: RwSignal::new(false),
            hint_text: RwSignal::new(None),
            hint_level: RwSignal::new(0),
            hint_target: RwSignal::new(None),
            hint_piece: RwSignal::new(None),
            refuted: RwSignal::new(false),
            wrong_at: RwSignal::new(None),
            pending_notice: StoredValue::new(None),
            moved: RwSignal::new(false),
            replaying: RwSignal::new(false),
            notice: RwSignal::new(None),
            reported: RwSignal::new(false),
            attempt: RwSignal::new(None),
            game_type: RwSignal::new(PuzzleGameTypeFilter::Any),
            difficulty: RwSignal::new(PuzzleDifficulty::Normal),
            length: RwSignal::new(PuzzleLength::Any),
            theme: RwSignal::new(None),
            collection: RwSignal::new(None),
            reveal_tokens: RwSignal::new(Vec::new()),
            reveal_states: StoredValue::new(Vec::new()),
            reveal_step: RwSignal::new(0),
            generation: StoredValue::new(0),
            started_at: StoredValue::new(0.0),
            solved_streak: RwSignal::new(0),
            opponent_move: RwSignal::new(None),
            setup_note: StoredValue::new(None),
            daily: RwSignal::new(None),
            playing_out: RwSignal::new(false),
            played_out: RwSignal::new(0),
            one_off_difficulty: StoredValue::new(None),
            reveal_armed: RwSignal::new(false),
            first_result: RwSignal::new(None),
            ever_solved: RwSignal::new(false),
            solved_before_replay: RwSignal::new(false),
            one_click: RwSignal::new(false),
            training: StoredValue::new(false),
        }
    }

    /// Reads the remembered filter; called after hydration so the server render matches.
    pub fn restore_preferences(&self) {
        self.game_type.set(load_game_type());
        self.difficulty.set(match load(DIFFICULTY_KEY).as_deref() {
            Some("Easier") => PuzzleDifficulty::Easier,
            Some("Harder") => PuzzleDifficulty::Harder,
            _ => PuzzleDifficulty::Normal,
        });
        self.length.set(match load(LENGTH_KEY).as_deref() {
            Some("One") => PuzzleLength::One,
            Some("Two") => PuzzleLength::Two,
            Some("ThreePlus") => PuzzleLength::ThreePlus,
            _ => PuzzleLength::Any,
        });
        self.one_click
            .set(load(ONE_CLICK_KEY).as_deref() == Some("1"));
    }

    /// False once the page is gone: replies to slow requests and timers must then do nothing.
    fn alive(&self) -> bool {
        self.generation.try_get_value().is_some()
    }

    pub fn solver(&self) -> Color {
        self.start
            .try_with_value(|start| start.as_ref().map(|state| state.turn_color))
            .flatten()
            .unwrap_or(Color::White)
    }

    fn lines(&self) -> Vec<Vec<String>> {
        self.puzzle
            .with_untracked(|puzzle| puzzle.as_ref().map(|p| p.lines.clone()))
            .unwrap_or_default()
    }

    fn is_current(&self, generation: u64) -> bool {
        self.generation.try_get_value() == Some(generation)
    }

    fn bump(&self) -> u64 {
        let generation = self.generation.get_value() + 1;
        self.generation.set_value(generation);
        generation
    }

    /// Shows `puzzle` from its start: the opponent's setup move is replayed briefly so the
    /// solver sees what just happened.
    pub fn load(&self, puzzle: PuzzleResponse) {
        if !self.alive() {
            return;
        }
        let generation = self.bump();
        let start = match start_state(
            &puzzle.game_type,
            puzzle.history.as_deref(),
            puzzle.position.as_deref(),
        ) {
            Ok(state) => state,
            Err(error) => {
                log::warn!("puzzle {} does not load: {error}", puzzle.nanoid);
                self.show_empty();
                return;
            }
        };
        remember_seen(&puzzle.nanoid);
        scroll_to_top();
        if self.training.get_value() {
            show_in_address_bar(&format!("/puzzles/{}{}", puzzle.nanoid, self.focus_query()));
        }
        let lines = puzzle.lines.clone();
        let setup = before_setup(&puzzle);
        self.start.set_value(Some(start.clone()));
        batch(|| {
            self.progress.set(PuzzleProgress::new(&lines));
            self.hinted.set(false);
            self.hint_text.set(None);
            self.notice.set(None);
            self.reported.set(false);
            self.hint_level.set(0);
            self.moved.set(false);
            self.replaying.set(false);
            self.opponent_move.set(None);
            self.setup_note.set_value(None);
            self.first_result.set(None);
            self.refuted.set(false);
            self.wrong_at.set(None);
            self.playing_out.set(false);
            self.played_out.set(0);
            self.reveal_armed.set(false);
            self.ever_solved.set(false);
            self.attempt.set(None);
            self.reveal_tokens.set(Vec::new());
            self.reveal_step.set(0);
            self.puzzle.set(Some(puzzle));
            self.game_state.clear_selection();
        });
        match setup {
            Some((before, token)) => {
                self.game_state.reset_with_state(before);
                self.phase.set(PuzzlePhase::Setup);
                let context = *self;
                set_timeout(
                    move || {
                        if !context.is_current(generation) {
                            return;
                        }
                        context.note_opponent(&token);
                        context
                            .setup_note
                            .set_value(context.opponent_move.get_untracked());
                        context.play_on_board(&token);
                        context.begin();
                    },
                    SETUP_DELAY,
                );
            }
            None => {
                self.game_state.reset_with_state(start);
                self.begin();
            }
        }
    }

    /// Nothing to show: the board and the old puzzle go away, not just the buttons.
    fn show_empty(&self) {
        if !self.alive() {
            return;
        }
        scroll_to_top();
        if self.training.get_value() {
            show_in_address_bar(&format!("/puzzles{}", self.focus_query()));
        }
        self.bump();
        batch(|| {
            self.puzzle.set(None);
            self.start.set_value(None);
            self.game_state
                .reset_with_game_type(hive_lib::GameType::MLP);
            self.phase.set(PuzzlePhase::Empty);
        });
    }

    fn begin(&self) {
        self.started_at.set_value(now_ms());
        self.phase.set(PuzzlePhase::Solving);
        if let Some(text) = self.pending_notice.try_update_value(Option::take).flatten() {
            self.tell(text);
        }
    }

    /// After a miss: the whole puzzle again from its start. Practice only, the result stands.
    pub fn retry(&self) {
        self.restart();
    }

    /// From the very start again (after a solve or the solution); never rated again.
    pub fn restart(&self) {
        self.bump();
        let Some(start) = self.start.get_value() else {
            return;
        };
        let lines = self.lines();
        batch(|| {
            self.game_state.reset_with_state(start);
            self.game_state.clear_selection();
            self.progress.set(PuzzleProgress::new(&lines));
            self.hint_text.set(None);
            self.notice.set(None);
            self.reveal_tokens.set(Vec::new());
            self.opponent_move.set(self.setup_note.get_value());
            self.refuted.set(false);
            self.wrong_at.set(None);
            self.playing_out.set(false);
            self.played_out.set(0);
            self.reveal_step.set(0);
            self.hint_level.set(0);
            if self.reported.get_untracked() {
                self.replaying.set(true);
                self.solved_before_replay
                    .set(self.ever_solved.get_untracked());
            }
            self.phase.set(PuzzlePhase::Solving);
        });
    }

    /// Moves on without an attempt: only before the first move or hint, so it reveals nothing.
    pub fn can_skip(&self) -> bool {
        self.phase.get() == PuzzlePhase::Solving
            && !self.moved.get()
            && !self.hinted.get()
            && !self.reported.get()
    }

    /// Shows a short explanation near the board and in the panel, then lets it fade.
    fn tell(&self, text: String) {
        self.notice.set(Some(text.clone()));
        let notice = self.notice;
        set_timeout(
            move || {
                if notice.try_get_untracked().flatten().as_deref() == Some(text.as_str()) {
                    let _ = notice.try_set(None);
                }
            },
            NOTICE_DURATION,
        );
    }

    fn play_on_board(&self, token: &str) {
        self.game_state.state().maybe_update(|state| {
            let already_passed = split_move(token).0 == "pass"
                && state
                    .history
                    .moves
                    .last()
                    .is_some_and(|(piece, _)| piece == "pass");
            if already_passed {
                return false;
            }
            if let Err(error) = play_token(state, token) {
                log::warn!("puzzle move {token} does not play: {error}");
            }
            true
        });
    }

    /// Judges the solver's move. A wrong move is shown, then the puzzle counts as failed.
    pub fn try_move(&self, piece: Piece, target: Position) {
        if self.phase.get_untracked() == PuzzlePhase::Failed && self.playing_out.get_untracked() {
            self.play_out(piece, target);
            return;
        }
        if self.phase.get_untracked() != PuzzlePhase::Solving {
            return;
        }
        let lines = self.lines();
        let before = self.game_state.state().get_untracked();
        let mut progress = self.progress.get_untracked();
        let verdict = progress.judge(&lines, &before, piece, target);
        self.moved.set(true);
        self.game_state.clear_selection();
        self.notice.set(None);
        self.game_state.play_turn(piece, target);
        self.progress.set(progress);
        match verdict {
            PuzzleVerdict::Continue { reply } => {
                self.phase.set(PuzzlePhase::Replying);
                let context = *self;
                let generation = self.generation.get_value();
                set_timeout(
                    move || {
                        if !context.is_current(generation) {
                            return;
                        }
                        context.note_opponent(&reply);
                        context.play_on_board(&reply);
                        context.hint_level.set(0);
                        context.hint_text.set(None);
                        context.phase.set(PuzzlePhase::Solving);
                    },
                    REPLY_DELAY,
                );
            }
            PuzzleVerdict::Solved => {
                self.phase.set(PuzzlePhase::Solved);
                if !self.reported.get_untracked() && !self.hinted.get_untracked() {
                    self.solved_streak.update(|streak| *streak += 1);
                } else if !self.reported.get_untracked() {
                    self.solved_streak.set(0);
                }
                self.ever_solved.set(true);
                self.report(true);
            }
            PuzzleVerdict::Wrong => {
                self.phase.set(PuzzlePhase::Failed);
                self.wrong_at.set(Some(target));
                self.solved_streak.set(0);
                self.report(false);
                self.refute();
            }
        }
    }

    /// The puzzle and the moves played on the board since its start, as the server expects them.
    fn moves_since_start(&self) -> Option<(String, String)> {
        let nanoid = self
            .puzzle
            .with_untracked(|puzzle| puzzle.as_ref().map(|p| p.nanoid.clone()))?;
        let from = self
            .start
            .with_value(|start| start.as_ref().map(|state| state.history.moves.len()))
            .unwrap_or(0);
        let moves = self.game_state.state().with_untracked(|state| {
            state.history.moves[from.min(state.history.moves.len())..]
                .iter()
                .map(|(piece, position)| format!("{piece} {position}"))
                .collect::<Vec<_>>()
                .join(";")
        });
        Some((nanoid, moves))
    }

    /// Asks the server for the answer that stops the win and plays it after a short pause, so the
    /// player sees why the move failed. Then the board stays open to play the position out.
    fn refute(&self) {
        let Some((nanoid, moves)) = self.moves_since_start() else {
            return;
        };
        let generation = self.generation.get_value();
        let context = *self;
        spawn_local(async move {
            let answer = puzzle_refutation(nanoid, moves).await;
            if !context.alive()
                || !context.is_current(generation)
                || context.phase.get_untracked() != PuzzlePhase::Failed
            {
                return;
            }
            let Ok(Some(answer)) = answer else {
                context.open_for_play_out();
                return;
            };
            set_timeout(
                move || {
                    if !context.is_current(generation)
                        || context.phase.get_untracked() != PuzzlePhase::Failed
                    {
                        return;
                    }
                    context.note_opponent(&answer);
                    context.play_on_board(&answer);
                    context.refuted.set(true);
                    context.open_for_play_out();
                },
                REPLY_DELAY,
            );
        });
    }

    fn open_for_play_out(&self) {
        let over = self
            .game_state
            .state()
            .with_untracked(|state| matches!(state.game_status, GameStatus::Finished(_)));
        self.playing_out.set(!over);
    }

    /// After a miss the player may keep going; the opponent answers with its stubbornest defence.
    fn play_out(&self, piece: Piece, target: Position) {
        self.game_state.clear_selection();
        self.notice.set(None);
        self.wrong_at.set(None);
        self.game_state.play_turn(piece, target);
        self.played_out.update(|moves| *moves += 1);
        self.playing_out.set(false);
        let (over, solver_again) = self.game_state.state().with_untracked(|state| {
            (
                matches!(state.game_status, GameStatus::Finished(_)),
                state.turn_color == self.solver(),
            )
        });
        if over {
            return;
        }
        if solver_again {
            self.note_opponent("pass");
            self.playing_out.set(true);
            return;
        }
        let Some((nanoid, moves)) = self.moves_since_start() else {
            return;
        };
        let generation = self.generation.get_value();
        let context = *self;
        spawn_local(async move {
            let answer = puzzle_defence(nanoid, moves).await;
            if !context.alive() || !context.is_current(generation) {
                return;
            }
            set_timeout(
                move || {
                    if !context.is_current(generation)
                        || context.phase.get_untracked() != PuzzlePhase::Failed
                    {
                        return;
                    }
                    if let Ok(Some(answer)) = answer {
                        context.note_opponent(&answer);
                        context.play_on_board(&answer);
                    }
                    context.open_for_play_out();
                },
                REPLY_DELAY,
            );
        });
    }

    fn report(&self, solved: bool) {
        if self.reported.get_untracked() {
            return;
        }
        self.reported.set(true);
        self.first_result.set(Some(solved));
        let Some(nanoid) = self
            .puzzle
            .with_untracked(|puzzle| puzzle.as_ref().map(|p| p.nanoid.clone()))
        else {
            return;
        };
        let hinted = self.hinted.get_untracked();
        let elapsed = (now_ms() - self.started_at.get_value()).max(0.0);
        let time_ms = Some(elapsed.min(f64::from(i32::MAX)) as i32);
        let context = *self;
        spawn_local(async move {
            let response = submit_puzzle_attempt(nanoid, solved, hinted, time_ms, true).await;
            if !context.alive() {
                return;
            }
            match response {
                Ok(response) => {
                    let counted = response.counted_play;
                    context.attempt.set(Some(response));
                    if counted {
                        context.puzzle.update(|puzzle| {
                            if let Some(puzzle) = puzzle {
                                puzzle.plays += 1;
                                puzzle.solves += i32::from(solved && !hinted);
                            }
                        });
                    }
                }
                Err(error) => log::warn!("could not record the puzzle attempt: {error}"),
            }
        });
    }

    /// Three steps, each giving away a little more: the kind of bug, then the piece with its
    /// moves, then the exact destination. Any of them counts as a miss on a rated try.
    pub fn hint(&self, costs_rating: bool) {
        if self.phase.get_untracked() != PuzzlePhase::Solving
            || self.hint_level.get_untracked() >= 3
        {
            return;
        }
        let lines = self.lines();
        let Some(token) = self
            .progress
            .with_untracked(|p| p.expected(&lines).cloned())
        else {
            return;
        };
        let state = self.game_state.state().get_untracked();
        let Ok(Some((piece, target))) = resolve_move(&state, &token) else {
            return;
        };
        let first_hint = !self.hinted.get_untracked();
        self.hinted.set(true);
        self.notice.set(None);
        let level = self.hint_level.get_untracked() + 1;
        self.hint_level.set(level);
        let from_reserve = state.board.position_of_piece(piece).is_none();
        let thrown = piece.color() != state.turn_color;
        self.hint_piece.set(Some(piece));
        if level == 1 {
            let idea = if thrown {
                "Think about a Pillbug throw.".to_string()
            } else if from_reserve {
                format!(
                    "Think about placing {}.",
                    with_article(bug_name(piece.bug()))
                )
            } else {
                format!(
                    "Think about moving {}.",
                    with_article(bug_name(piece.bug()))
                )
            };
            let cost = if costs_rating && first_hint {
                " This try now counts as a miss."
            } else {
                ""
            };
            self.hint_text.set(Some(format!("{idea}{cost}")));
            return;
        }
        match state.board.position_of_piece(piece) {
            Some(position) => self.game_state.show_moves(piece, position),
            None => self
                .game_state
                .show_spawns(piece, Position::initial_spawn_position()),
        }
        if level == 2 {
            self.hint_text
                .set(Some(describe_piece(piece, from_reserve, thrown)));
        } else {
            self.game_state.move_info().update(|info| {
                info.target_positions = vec![target];
            });
            self.hint_target.set(Some(target));
            let bug = bug_name(piece.bug());
            self.hint_text.set(Some(if thrown {
                format!("Throw the {bug} to the highlighted spot.")
            } else if from_reserve {
                format!("Place your {bug} on the highlighted spot.")
            } else {
                format!("Move the {bug} to the highlighted spot.")
            }));
        }
    }

    /// Plays the rest of the main line out on the board, one move at a time, with the moves
    /// spelled out. Counts as a miss on a rated try.
    pub fn reveal(&self) {
        let phase = self.phase.get_untracked();
        if matches!(
            phase,
            PuzzlePhase::Revealed | PuzzlePhase::Loading | PuzzlePhase::Empty
        ) {
            return;
        }
        if phase == PuzzlePhase::Failed {
            self.retry();
        }
        self.report(false);
        if phase != PuzzlePhase::Solved {
            self.solved_streak.set(0);
        }
        self.phase.set(PuzzlePhase::Revealed);
        self.game_state.clear_selection();
        self.notice.set(None);
        let lines = self.lines();
        let (index, line) = self.progress.with_untracked(|progress| {
            (
                progress.index,
                progress.open_lines.first().copied().unwrap_or(0),
            )
        });
        let (Some(tokens), Some(mut state)) = (lines.get(line).cloned(), self.start.get_value())
        else {
            return;
        };
        let mut states = vec![state.clone()];
        let mut described = Vec::new();
        for token in &tokens {
            described.push(describe_move(&state, token));
            if play_token(&mut state, token).is_err() {
                break;
            }
            states.push(state.clone());
        }
        let steps = states.len() - 1;
        let index = index.min(steps);
        self.reveal_states.set_value(states);
        self.reveal_tokens.set(described);
        self.reveal_step.set(index);
        let generation = self.generation.get_value();
        for step in index + 1..=steps {
            let context = *self;
            set_timeout(
                move || {
                    if context.is_current(generation) {
                        context.show_solution_step(step, false);
                    }
                },
                SOLUTION_STEP * (step - index) as u32,
            );
        }
    }

    /// Puts the board at `step` of the solution (0 = before it). `manual` stops the autoplay.
    pub fn show_solution_step(&self, step: usize, manual: bool) {
        if manual {
            self.bump();
        }
        let Some(state) = self
            .reveal_states
            .with_value(|states| states.get(step).cloned())
        else {
            return;
        };
        self.game_state.reset_with_state(state);
        self.reveal_step.set(step);
    }

    pub fn solution_steps(&self) -> usize {
        self.reveal_states
            .with_value(|states| states.len().saturating_sub(1))
    }

    /// Leaves an untouched puzzle for the next one, without an attempt.
    pub fn skip(&self, next: Callback<()>) {
        if self.can_skip() {
            self.pending_notice
                .set_value(Some("Skipped the last one, not rated.".to_string()));
            next.run(());
        }
    }

    /// Fetches and shows the next unseen puzzle near the player's rating.
    pub fn load_next(&self) {
        self.phase.set(PuzzlePhase::Loading);
        let context = *self;
        let game_type = self.game_type.get_untracked();
        let difficulty = self
            .one_off_difficulty
            .try_update_value(Option::take)
            .flatten()
            .unwrap_or_else(|| self.difficulty.get_untracked());
        let length = self.length.get_untracked();
        let theme = self.theme.get_untracked().unwrap_or_default();
        let collection = self.collection.get_untracked().unwrap_or_default();
        spawn_local(async move {
            let found = next_puzzle(
                game_type,
                difficulty,
                length,
                theme,
                collection,
                seen_puzzles().join(","),
            )
            .await;
            if !context.alive() {
                return;
            }
            match found {
                Ok(Some(puzzle)) => context.load(puzzle),
                Ok(None) => context.show_empty(),
                Err(error) => {
                    log::warn!("could not fetch a puzzle: {error}");
                    context.show_empty();
                }
            }
        });
    }

    pub fn set_difficulty(&self, difficulty: PuzzleDifficulty) {
        self.difficulty.set(difficulty);
        store(DIFFICULTY_KEY, &format!("{difficulty:?}"));
        self.note_new_selection();
    }

    pub fn set_length(&self, length: PuzzleLength) {
        self.length.set(length);
        store(LENGTH_KEY, &format!("{length:?}"));
        self.note_new_selection();
    }

    /// The set being trained as a query string, so links and reloads keep it.
    pub fn focus_query(&self) -> String {
        match (self.collection.get_untracked(), self.theme.get_untracked()) {
            (Some(collection), _) => format!("?collection={collection}"),
            (None, Some(theme)) => format!("?theme={theme}"),
            (None, None) => String::new(),
        }
    }

    pub fn set_focus(&self, theme: Option<String>, collection: Option<String>) {
        self.theme.set(theme);
        self.collection.set(collection);
    }

    pub fn leave_focus(&self) {
        self.set_focus(None, None);
        self.training.set_value(true);
        if let Some(nanoid) = self
            .puzzle
            .with_untracked(|puzzle| puzzle.as_ref().map(|p| p.nanoid.clone()))
        {
            show_in_address_bar(&format!("/puzzles/{nanoid}"));
        }
    }

    fn note_new_selection(&self) {
        if self.phase.get_untracked() == PuzzlePhase::Solving {
            self.tell("Applies from the next puzzle.".to_string());
        }
    }

    pub fn set_one_click(&self, one_click: bool) {
        self.one_click.set(one_click);
        store(ONE_CLICK_KEY, if one_click { "1" } else { "0" });
    }

    fn note_opponent(&self, token: &str) {
        // The engine passes for a side with no move as soon as the move before it is played, so by
        // now the turn is already back with the solver.
        let note = if split_move(token).0 == "pass" {
            format!(
                "{} had to pass.",
                color_label(self.solver().opposite_color())
            )
        } else {
            let state = self.game_state.state().get_untracked();
            last_move_note(&state, token)
        };
        self.opponent_move.set(Some(note));
    }

    /// Show solution with a safety catch: on a rated try the first press only arms it. After a
    /// hint the try is already a miss, so there is nothing left to guard.
    pub fn request_reveal(&self, costs_rating: bool) {
        if costs_rating && !self.hinted.get_untracked() && !self.reveal_armed.get_untracked() {
            self.reveal_armed.set(true);
            return;
        }
        self.reveal_armed.set(false);
        self.reveal();
    }

    /// Whether a hint or the solution would still cost rating on this try.
    pub fn costs_rating(&self, signed_in: bool) -> bool {
        signed_in
            && !self.reported.get()
            && !self.replaying.get()
            && !self
                .puzzle
                .with(|current| current.as_ref().is_some_and(|p| p.attempted))
    }

    /// Leaves the puzzle at any point: free before trying (or when nothing is rated any more),
    /// otherwise it counts as a miss first.
    pub fn give_up(&self) {
        if self.phase.get_untracked() == PuzzlePhase::Solving {
            self.reveal();
        }
    }

    pub fn set_game_type(&self, game_type: PuzzleGameTypeFilter) {
        self.game_type.set(game_type);
        self.note_new_selection();
        store(GAME_TYPE_KEY, &format!("{game_type:?}"));
    }

    /// Whether the side to move on the board is the solver and the puzzle is live.
    pub fn solver_to_move(&self) -> bool {
        (self.phase.get() == PuzzlePhase::Solving
            || (self.phase.get() == PuzzlePhase::Failed && self.playing_out.get()))
            && self.game_state.state().with(|state| state.turn_color) == self.solver()
    }

    /// A selected piece with nowhere to go gets a reason instead of silence.
    fn explain_stuck(&self, piece: Piece, position: Position) {
        let state = self.game_state.state().get_untracked();
        if state.board.is_pinned(piece) && state.board.top_piece(position) == Some(piece) {
            self.tell(format!(
                "This {} is pinned: moving it would split the hive.",
                bug_name(piece.bug())
            ));
        }
    }
}

fn scroll_to_top() {
    #[cfg(not(feature = "ssr"))]
    if let Some(window) = web_sys::window() {
        window.scroll_to_with_x_and_y(0.0, 0.0);
    }
    #[cfg(not(feature = "ssr"))]
    if let Some(panel) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| {
            document
                .query_selector(".ui-board-side-panel")
                .ok()
                .flatten()
        })
    {
        panel.set_scroll_top(0);
    }
}

fn show_in_address_bar(path: &str) {
    #[cfg(not(feature = "ssr"))]
    if let Some(history) = web_sys::window().and_then(|window| window.history().ok()) {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(path));
    }
    #[cfg(feature = "ssr")]
    let _ = path;
}

/// The state before the setup move and the move itself, when the puzzle has one.
fn before_setup(puzzle: &PuzzleResponse) -> Option<(State, String)> {
    let history = puzzle.history.as_deref()?;
    let tokens: Vec<&str> = history
        .split(';')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    let (last, earlier) = tokens.split_last()?;
    let earlier: String = earlier.iter().map(|token| format!("{token};")).collect();
    let before = State::new_from_str(&earlier, &puzzle.game_type).ok()?;
    if split_move(last).0 == "pass" {
        return None;
    }
    Some((before, (*last).to_string()))
}

fn with_article(noun: &str) -> String {
    match noun.chars().next() {
        Some('A' | 'E' | 'I' | 'O' | 'U') => format!("an {noun}"),
        _ => format!("a {noun}"),
    }
}

fn bug_name(bug: Bug) -> &'static str {
    match bug {
        Bug::Ant => "Ant",
        Bug::Beetle => "Beetle",
        Bug::Grasshopper => "Grasshopper",
        Bug::Ladybug => "Ladybug",
        Bug::Mosquito => "Mosquito",
        Bug::Pillbug => "Pillbug",
        Bug::Queen => "Queen",
        Bug::Spider => "Spider",
    }
}

fn describe_piece(piece: Piece, from_reserve: bool, thrown: bool) -> String {
    let bug = bug_name(piece.bug());
    if thrown {
        format!("Throw this {bug} with a Pillbug move.")
    } else if from_reserve {
        format!("Place your {bug}.")
    } else {
        format!("Move this {bug}.")
    }
}

fn now_ms() -> f64 {
    #[cfg(not(feature = "ssr"))]
    {
        web_sys::js_sys::Date::now()
    }
    #[cfg(feature = "ssr")]
    {
        0.0
    }
}

fn storage() -> Option<web_sys::Storage> {
    #[cfg(not(feature = "ssr"))]
    {
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    }
    #[cfg(feature = "ssr")]
    {
        None
    }
}

fn store(key: &str, value: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(key, value);
    }
}

fn load(key: &str) -> Option<String> {
    storage().and_then(|storage| storage.get_item(key).ok().flatten())
}

fn load_game_type() -> PuzzleGameTypeFilter {
    match load(GAME_TYPE_KEY).as_deref() {
        Some("Mlp") => PuzzleGameTypeFilter::Mlp,
        Some("Base") => PuzzleGameTypeFilter::Base,
        _ => PuzzleGameTypeFilter::Any,
    }
}

/// Recently shown puzzles, so a guest is not served the same one twice in a row.
fn seen_puzzles() -> Vec<String> {
    load(GUEST_SEEN_KEY)
        .map(|seen| seen.split(',').map(str::to_string).collect())
        .unwrap_or_default()
}

fn remember_seen(nanoid: &str) {
    let mut seen = seen_puzzles();
    seen.retain(|id| id != nanoid && !id.is_empty());
    seen.push(nanoid.to_string());
    let skip = seen.len().saturating_sub(GUEST_SEEN_LIMIT);
    store(GUEST_SEEN_KEY, &seen[skip..].join(","));
}

/// Board interaction for puzzles: the solver selects and plays as in analysis, but the move goes
/// to the puzzle judge instead of the game.
pub fn puzzle_hiveground_interaction(puzzle: PuzzleContext) -> HivegroundInteraction {
    let game_state = puzzle.game_state;
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let capabilities = Signal::derive(move || {
        if puzzle.solver_to_move() {
            let drag_color = config
                .with(|config| config.drag_and_drop)
                .then(|| puzzle.solver());
            HivegroundCapabilities {
                drag_color,
                ..HivegroundCapabilities::analysis_selection()
            }
        } else {
            HivegroundCapabilities::board_inspection()
        }
    });
    let select_target = move |position: Position| {
        let was_selected = game_state
            .move_info()
            .with_untracked(|move_info| move_info.target_position == Some(position));
        game_state.set_target(position);
        let confirm = current_confirm.get_untracked();
        if confirm == MoveConfirm::Single || (confirm == MoveConfirm::Double && was_selected) {
            let active = game_state
                .move_info()
                .with_untracked(|move_info| move_info.active.map(|(piece, _)| piece));
            if let Some(piece) = active {
                puzzle.try_move(piece, position);
            }
        }
    };
    let dispatch = Callback::new(move |action: HivegroundAction| {
        if !matches!(
            action,
            HivegroundAction::ResetSelection | HivegroundAction::PreselectPiece { .. }
        ) {
            puzzle.reveal_armed.set(false);
        }
        if !puzzle.solver_to_move() {
            return;
        }
        match action {
            HivegroundAction::SelectBoardPiece { piece, position } => {
                game_state.show_moves(piece, position);
                let stuck = game_state
                    .move_info()
                    .with_untracked(|move_info| move_info.target_positions.is_empty());
                if stuck && piece.color() == puzzle.solver() {
                    puzzle.explain_stuck(piece, position);
                } else {
                    puzzle.notice.set(None);
                }
            }
            HivegroundAction::SelectReservePiece { piece, position } => {
                game_state.show_spawns(piece, position)
            }
            HivegroundAction::SelectTarget { position } => select_target(position),
            HivegroundAction::DropOnTarget {
                position,
                dragged,
                dragged_type,
            } => {
                let (is_target, drags_selection) =
                    game_state.move_info().with_untracked(|move_info| {
                        (
                            move_info.target_positions.contains(&position),
                            move_info.active.is_some_and(|(selected, selected_type)| {
                                same_piece(selected, selected_type, dragged, dragged_type)
                            }),
                        )
                    });
                if is_target && drags_selection {
                    select_target(position);
                } else {
                    game_state.clear_selection();
                }
            }
            HivegroundAction::ResetSelection => {
                let keeps_hint = puzzle.hint_level.get_untracked() >= 2
                    && game_state.move_info().with_untracked(|move_info| {
                        move_info.active.map(|(piece, _)| piece)
                            == puzzle.hint_piece.get_untracked()
                    });
                if !keeps_hint {
                    game_state.clear_selection();
                }
            }
            HivegroundAction::PreselectPiece { .. } => {}
        }
    });
    HivegroundInteraction::new(
        capabilities,
        HivegroundActions {
            dispatch: Some(dispatch),
        },
    )
}

// Reserve selections pick the lowest-numbered piece of a bug, so a dragged reserve piece matches
// on bug and colour.
fn same_piece(
    selected: Piece,
    selected_type: PieceType,
    dragged: Piece,
    dragged_type: PieceType,
) -> bool {
    match (
        selected_type == PieceType::Board,
        dragged_type == PieceType::Board,
    ) {
        (true, true) => selected == dragged,
        (false, false) => selected.color() == dragged.color() && selected.bug() == dragged.bug(),
        _ => false,
    }
}

/// Signed-in state that is false until the page has hydrated: the server renders before it
/// knows the identity, and the first client render must match it.
pub fn is_signed_in() -> Signal<bool> {
    let auth = expect_context::<AuthContext>();
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    Signal::derive(move || {
        mounted.get() && matches!(auth.identity.get(), Some(AuthIdentity::User(_)))
    })
}

fn color_label(color: Color) -> &'static str {
    match color {
        Color::White => "White",
        Color::Black => "Black",
    }
}

fn direction_label(direction: hive_lib::Direction) -> &'static str {
    match direction {
        hive_lib::Direction::E => "right of",
        hive_lib::Direction::W => "left of",
        hive_lib::Direction::NE => "above right of",
        hive_lib::Direction::NW => "above left of",
        hive_lib::Direction::SE => "below right of",
        hive_lib::Direction::SW => "below left of",
    }
}

/// A move in words, e.g. "White Beetle onto the Black Queen" or "Black Ant to the right of the
/// White Spider", described from the board rather than the notation (which may name the moving
/// piece itself as the landmark).
fn last_move_note(state: &State, token: &str) -> String {
    let Ok(Some((piece, _))) = resolve_move(state, token) else {
        return format!("{} had to pass.", color_label(state.turn_color));
    };
    if piece.color() != state.turn_color {
        return format!(
            "{} threw the {} {} with a Pillbug.",
            color_label(state.turn_color),
            color_label(piece.color()),
            bug_name(piece.bug())
        );
    }
    let verb = if state.board.position_of_piece(piece).is_some() {
        "moved"
    } else {
        "placed"
    };
    format!(
        "{} {verb} their {}.",
        color_label(piece.color()),
        bug_name(piece.bug())
    )
}

fn describe_move(state: &State, token: &str) -> String {
    let Ok(Some((piece, target))) = resolve_move(state, token) else {
        return "Pass (no legal move)".to_string();
    };
    let board = &state.board;
    let name = |piece: Piece| format!("{} {}", color_label(piece.color()), bug_name(piece.bug()));
    let from = board.position_of_piece(piece);
    let visible_beside = |cell: Position| match board.top_piece(cell) {
        Some(top) if top == piece => board.under_piece(cell),
        top => top,
    };
    let one_of_a_kind = |landmark: Piece| {
        !matches!(
            landmark.bug(),
            Bug::Ant | Bug::Beetle | Bug::Grasshopper | Bug::Spider
        ) || (1..=3).all(|order| {
            let other = Piece::new_from(landmark.bug(), landmark.color(), order);
            other == landmark || board.position_of_piece(other).is_none()
        })
    };
    let twin_of_mover =
        |landmark: Piece| landmark.bug() == piece.bug() && landmark.color() == piece.color();
    let near = |cell: Position| {
        let landmarks: Vec<(Piece, Position)> = cell
            .positions_around()
            .filter_map(|around| visible_beside(around).map(|landmark| (landmark, around)))
            .collect();
        landmarks
            .iter()
            .find(|(landmark, _)| !twin_of_mover(*landmark) && one_of_a_kind(*landmark))
            .or_else(|| {
                landmarks
                    .iter()
                    .find(|(landmark, _)| !twin_of_mover(*landmark))
            })
            .or(landmarks.first())
            .copied()
            .map(|(landmark, at)| {
                format!(
                    "{} the {}",
                    direction_label(at.direction(cell)),
                    name(landmark)
                )
            })
    };
    let Some(from) = from else {
        return match board
            .top_piece(target)
            .map(|under| format!("onto the {}", name(under)))
            .or_else(|| near(target))
        {
            Some(place) => format!("Place {} {place}", name(piece)),
            None => format!("Place {}", name(piece)),
        };
    };
    let verb = if piece.color() != state.turn_color {
        "Throw"
    } else {
        "Move"
    };
    let twin_on_board = matches!(
        piece.bug(),
        Bug::Ant | Bug::Beetle | Bug::Grasshopper | Bug::Spider
    ) && (1..=3).any(|order| {
        let twin = Piece::new_from(piece.bug(), piece.color(), order);
        twin != piece && board.position_of_piece(twin).is_some()
    });
    let origin = twin_on_board.then(|| match board.under_piece(from) {
        Some(under) => Some(format!("from on top of the {}", name(under))),
        None => near(from).map(|place| format!("from {place}")),
    });
    let destination = board
        .top_piece(target)
        .map(|under| format!("onto the {}", name(under)))
        .or_else(|| near(target));
    match (origin.flatten(), destination) {
        (Some(origin), Some(place)) if place.starts_with("onto") => {
            format!("{verb} the {} {origin} {place}", name(piece))
        }
        (Some(origin), Some(place)) => format!("{verb} the {} {origin} to {place}", name(piece)),
        (Some(origin), None) => format!("{verb} the {} {origin}", name(piece)),
        (None, Some(place)) => format!("{verb} {} {place}", name(piece)),
        (None, None) => format!("{verb} {}", name(piece)),
    }
}
