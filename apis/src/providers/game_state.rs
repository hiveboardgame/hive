use std::{ops::Deref, str::FromStr};

use crate::{
    common::{MoveInfo, PieceType},
    responses::GameResponse,
};
use hive_lib::{Color, GameControl, GameStatus, GameType, Piece, Position, State, Turn};
use leptos::{logging::log, prelude::*, reactive::effect::batch};
use reactive_stores::Store;
use shared_types::{GameId, Takeback};
use uuid::Uuid;

use super::{
    analysis::AnalysisContext,
    api_requests::ApiRequests,
    auth_context::{AuthContext, AuthIdentity},
    ApiRequestsProvider,
};

#[derive(Clone, Copy, Debug)]
pub struct GameStateStore(Store<GameState>);

impl Deref for GameStateStore {
    type Target = Store<GameState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for GameStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateStore {
    pub fn new() -> Self {
        Self(Store::new(GameState::new()))
    }

    pub fn full_reset(&self) {
        self.replace(GameState::new());
    }

    pub fn reset_with_game_type(&self, game_type: GameType) {
        self.replace(GameState::new_with_game_type(game_type));
    }

    pub fn reset_with_state(&self, state: State) {
        let mut game_state = GameState::new_with_game_type(state.game_type);
        game_state.state = state;
        self.replace(game_state);
    }

    pub(crate) fn reset_from_response(&self, game: &GameResponse) {
        self.replace(GameState::from_response(game));
    }

    pub(crate) fn replace(&self, game_state: GameState) {
        self.0.set(game_state);
    }

    pub fn user_color_as_signal(
        &self,
        identity: Signal<Option<AuthIdentity>>,
    ) -> Signal<Option<Color>> {
        let white_id = self.white_id();
        let black_id = self.black_id();
        Memo::new(move |_| {
            let user_id = identity.get().and_then(AuthIdentity::user_id);
            color_for_user(user_id, white_id.get(), black_id.get())
        })
        .into()
    }

    pub fn user_color_untracked(&self, user_id: Option<Uuid>) -> Option<Color> {
        color_for_user(
            user_id,
            self.white_id().get_untracked(),
            self.black_id().get_untracked(),
        )
    }

    pub fn send_game_control(&self, game_control: GameControl, user: Uuid) {
        let api = expect_context::<ApiRequestsProvider>().0;
        if let Some(color) = self.user_color_untracked(Some(user)) {
            if color != game_control.color() {
                log!("This is a bug, you should only send GCs of your own color, user id color is {color} and gc color is {}", game_control.color());
            } else if let Some(game_id) = self.game_id().get_untracked() {
                api.get().game_control(game_id, game_control);
            } else {
                log!("This is a bug, there should be a game_id");
            }
        }
    }

    pub fn play_turn(&self, piece: Piece, position: Position) {
        self.state().maybe_update(
            |state| match state.play_turn_from_position(piece, position) {
                Ok(()) => true,
                Err(error) => {
                    log!("Could not play turn: {} {} {}", piece, position, error);
                    false
                }
            },
        );
    }

    pub fn clear_selection(&self) {
        self.move_info().maybe_update(|move_info| {
            if *move_info == MoveInfo::new() {
                false
            } else {
                move_info.reset();
                true
            }
        })
    }

    pub fn move_active(&self, analysis: Option<AnalysisContext>, api: ApiRequests) {
        let (active, position) = self.move_info().with_untracked(|move_info| {
            (
                move_info.active.map(|(piece, _)| piece),
                move_info.target_position,
            )
        });
        let (Some(active), Some(position)) = (active, position) else {
            return;
        };
        let was_at_history_edge = self.board_view().with_untracked(|view| view.is_history())
            && self.is_last_turn_untracked();

        batch(|| {
            let played = self
                .state()
                .try_maybe_update(|state| {
                    let previous_len = state.history.moves.len();
                    if let Err(error) = state.play_turn_from_position(active, position) {
                        log!("Could not play turn: {} {} {}", active, position, error);
                        return (false, None);
                    }
                    let appended = if analysis.is_some() {
                        state.history.moves[previous_len..]
                            .iter()
                            .cloned()
                            .zip(state.hashes[previous_len..].iter().copied())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    (true, Some((appended, state.turn_color, state.turn)))
                })
                .flatten();
            let Some((appended, turn_color, turn)) = played else {
                return;
            };

            if let Some(analysis) = analysis {
                analysis.store.append_moves(appended, *self);
                self.clear_selection();
                analysis.sync_reserve.run(turn_color);
            } else if let Some(game_id) = self.game_id().get_untracked() {
                api.turn(game_id, Turn::Move(active, position));
                self.clear_selection();
                if was_at_history_edge {
                    self.board_view().set(BoardView::History {
                        turn: turn.checked_sub(1),
                    });
                }
            }
        });
    }

    pub fn is_move_allowed(&self, in_analysis: bool) -> bool {
        if in_analysis {
            return true;
        }
        let user_id = expect_context::<AuthContext>()
            .identity
            .get_untracked()
            .and_then(AuthIdentity::user_id);
        let user_color = self.user_color_untracked(user_id);
        self.state().with_untracked(|state| {
            live_move_allowed(user_color, state.turn_color, &state.game_status)
        })
    }

    pub fn show_moves(&self, piece: Piece, position: Position) {
        let target_positions = self.state().with_untracked(|state| {
            state
                .board
                .moves(state.turn_color)
                .get(&(piece, position))
                .cloned()
        });
        self.move_info().update(|move_info| {
            move_info.reset();
            move_info.current_position = Some(position);
            if let Some(target_positions) = target_positions {
                move_info.target_positions = target_positions;
                move_info.active = Some((piece, PieceType::Board));
            }
        });
    }

    pub fn show_spawns(&self, piece: Piece, position: Position) {
        let (target_positions, active) = self.state().with_untracked(|state| {
            let target_positions = state
                .board
                .spawnable_positions(state.turn_color)
                .collect::<Vec<Position>>();
            let active = state
                .board
                .reserve(state.turn_color, state.game_type)
                .get(&piece.bug())
                .and_then(|pieces| pieces.first())
                .and_then(|piece| Piece::from_str(piece).ok());
            (target_positions, active)
        });
        self.move_info().update(|move_info| {
            move_info.reset();
            move_info.target_positions = target_positions;
            if let Some(piece) = active {
                move_info.active = Some((piece, PieceType::Reserve));
                move_info.reserve_position = Some(position);
            }
        });
    }

    pub fn set_target(&self, position: Position) {
        self.move_info().maybe_update(|move_info| {
            if move_info.target_position == Some(position) {
                false
            } else {
                move_info.target_position = Some(position);
                true
            }
        })
    }

    pub fn show_history_turn(&self, turn: usize) {
        self.board_view()
            .set(BoardView::History { turn: Some(turn) });
    }

    pub fn view_game(&self) {
        self.board_view().set(BoardView::Live);
    }

    pub fn view_history(&self) {
        if self
            .board_view()
            .with_untracked(|view| matches!(view, BoardView::Live))
        {
            let turn = self
                .state()
                .with_untracked(|state| state.turn.checked_sub(1));
            self.board_view().set(BoardView::History { turn });
        }
    }

    pub fn set_game_response(&self, game_response: GameResponse) {
        self.game_response().set(Some(game_response));
    }

    pub fn is_finished(&self) -> Memo<bool> {
        let state = self.state();
        let game_response = self.game_response();
        Memo::new(move |_| {
            let state_is_finished = state.with(|state| {
                matches!(
                    state.game_status,
                    GameStatus::Finished(_) | GameStatus::Adjudicated
                )
            });
            state_is_finished
                || game_response
                    .with(|response| response.as_ref().is_some_and(|game| game.finished))
        })
    }

    pub fn is_last_turn_as_signal(&self) -> Signal<bool> {
        let state = self.state();
        let board_view = self.board_view();
        Memo::new(move |_| {
            let state_turn = state.with(|state| state.turn);
            board_view.with(|view| view.is_last_turn(state_turn))
        })
        .into()
    }

    pub fn is_last_turn_untracked(&self) -> bool {
        let state_turn = self.state().with_untracked(|state| state.turn);
        self.board_view()
            .with_untracked(|view| view.is_last_turn(state_turn))
    }

    pub fn takeback_allowed(&self) -> bool {
        self.game_response().with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(takeback_allowed_for_response)
        })
    }
}

pub fn provide_game_state() {
    provide_context(GameStateStore::new());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardView {
    Live,
    History {
        /// Zero-based move index; `None` is the initial empty position.
        turn: Option<usize>,
    },
}

impl BoardView {
    pub fn displayed_turn(self, state_turn: usize) -> Option<usize> {
        match self {
            Self::Live => state_turn.checked_sub(1),
            Self::History { turn } => turn,
        }
    }

    pub fn is_history(self) -> bool {
        matches!(self, Self::History { .. })
    }

    pub fn is_last_turn(self, state_turn: usize) -> bool {
        self.displayed_turn(state_turn) == state_turn.checked_sub(1)
    }
}

pub(crate) fn color_for_user(
    user_id: Option<Uuid>,
    white_id: Option<Uuid>,
    black_id: Option<Uuid>,
) -> Option<Color> {
    let user_id = user_id?;
    if Some(user_id) == white_id {
        Some(Color::White)
    } else if Some(user_id) == black_id {
        Some(Color::Black)
    } else {
        None
    }
}

pub(crate) fn live_move_allowed(
    user_color: Option<Color>,
    turn_color: Color,
    status: &GameStatus,
) -> bool {
    user_color == Some(turn_color)
        && !matches!(status, GameStatus::Finished(_) | GameStatus::Adjudicated)
}

#[derive(Clone, Debug, Store)]
pub struct GameState {
    // game_id is the nanoid of the game
    pub game_id: Option<GameId>,
    // the gamestate
    pub state: State,
    pub black_id: Option<Uuid>,
    pub white_id: Option<Uuid>,
    pub move_info: MoveInfo,
    pub board_view: BoardView,
    // Unanswered game_control
    pub game_control_pending: Option<GameControl>,
    pub game_response: Option<GameResponse>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub(crate) fn from_response(game: &GameResponse) -> Self {
        let game_control_pending = if game.finished {
            None
        } else {
            game.game_control_history
                .last()
                .and_then(|(_, control)| match control {
                    GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => Some(*control),
                    _ => None,
                })
        };
        Self {
            game_id: Some(game.game_id.clone()),
            state: game.create_state(),
            black_id: Some(game.black_player.uid),
            white_id: Some(game.white_player.uid),
            move_info: MoveInfo::new(),
            board_view: BoardView::Live,
            game_control_pending,
            game_response: Some(game.clone()),
        }
    }

    pub fn new() -> Self {
        Self::new_with_game_type(GameType::MLP)
    }

    pub fn new_with_game_type(game_type: GameType) -> Self {
        let state = State::new(game_type, false);
        Self {
            game_id: None,
            state,
            black_id: None,
            white_id: None,
            move_info: MoveInfo::new(),
            board_view: BoardView::Live,
            game_control_pending: None,
            game_response: None,
        }
    }
}

fn takeback_allowed_for_response(game_response: &GameResponse) -> bool {
    let color_allowed = |color: &Color, game_response: &GameResponse| {
        let rated = game_response.rated;
        let takeback = match color {
            Color::Black => &game_response.black_player.takeback,
            Color::White => &game_response.white_player.takeback,
        };
        takeback == &Takeback::Always || takeback == &Takeback::CasualOnly && !rated
    };
    let white = color_allowed(&Color::Black, game_response);
    let black = color_allowed(&Color::White, game_response);
    white && black
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::responses::UserResponse;
    use chrono::Utc;
    use hive_lib::Direction as BoardDirection;
    use leptos::prelude::Owner;
    use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};
    use std::collections::HashMap;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    fn with_store(test: impl FnOnce(GameStateStore)) {
        let owner = Owner::new();
        owner.with(|| test(GameStateStore::new()));
    }

    #[test]
    fn user_color_handles_partial_player_data() {
        let white = Uuid::new_v4();
        let black = Uuid::new_v4();
        let spectator = Uuid::new_v4();

        let cases = [
            (Some(white), Some(white), None, Some(Color::White)),
            (Some(black), None, Some(black), Some(Color::Black)),
            (Some(white), Some(white), Some(black), Some(Color::White)),
            (Some(black), Some(white), Some(black), Some(Color::Black)),
            (Some(spectator), Some(white), Some(black), None),
            (None, Some(white), Some(black), None),
        ];

        for (user_id, white_id, black_id, expected) in cases {
            assert_eq!(color_for_user(user_id, white_id, black_id), expected);
        }
    }

    fn game_response() -> GameResponse {
        let player = |username: &str| UserResponse {
            username: username.to_string(),
            uid: Uuid::new_v4(),
            patreon: false,
            bot: false,
            admin: false,
            deleted: false,
            ratings: HashMap::new(),
            takeback: Takeback::Always,
            lang: None,
        };
        let now = Utc::now();
        GameResponse {
            uuid: Uuid::new_v4(),
            game_id: GameId("response-game".to_string()),
            tournament: None,
            current_player_id: Uuid::new_v4(),
            turn: 0,
            finished: false,
            game_status: GameStatus::InProgress,
            game_type: GameType::Base,
            tournament_queen_rule: false,
            white_player: player("white"),
            black_player: player("black"),
            moves: HashMap::new(),
            spawns: Vec::new(),
            rated: false,
            reserve_black: HashMap::new(),
            reserve_white: HashMap::new(),
            history: Vec::new(),
            game_control_history: Vec::new(),
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            time_mode: TimeMode::Untimed,
            time_base: None,
            time_increment: None,
            speed: GameSpeed::Untimed,
            black_time_left: None,
            white_time_left: None,
            last_interaction: None,
            created_at: now,
            updated_at: now,
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown,
            repetitions: Vec::new(),
            game_start: GameStart::Immediate,
            game_speed: GameSpeed::Untimed,
            move_times: Vec::new(),
            tournament_game_result: TournamentGameResult::Unknown,
        }
    }

    fn game_response_with_valid_history(game_id: &str) -> GameResponse {
        let mut response = game_response();
        let mut state = State::new(GameType::Base, true);
        state
            .play_turn_from_history("wS1", "")
            .expect("valid opening");
        state
            .play_turn_from_history("bG1", "/wS1")
            .expect("valid reply");

        response.game_id = GameId(game_id.to_string());
        response.current_player_id = match state.turn_color {
            Color::White => response.white_player.uid,
            Color::Black => response.black_player.uid,
        };
        response.turn = state.turn;
        response.tournament_queen_rule = state.tournament;
        response.history = state.history.moves;
        response.hashes = state.hashes;
        response
    }

    fn dirty_game_a(game_state: GameStateStore) {
        let response = game_response_with_valid_history("game-a");
        game_state.reset_from_response(&response);
        game_state.move_info().set(MoveInfo {
            active: Some((piece("wA1"), PieceType::Reserve)),
            current_position: None,
            target_positions: vec![Position::initial_spawn_position()],
            target_position: Some(Position::initial_spawn_position()),
            reserve_position: Some(Position::new(0, 0)),
        });
        game_state
            .board_view()
            .set(BoardView::History { turn: Some(0) });
        game_state
            .game_control_pending()
            .set(Some(GameControl::TakebackRequest(Color::Black)));
    }

    fn assert_authoritative_snapshot(
        game_state: GameStateStore,
        response: &GameResponse,
        pending_control: Option<GameControl>,
    ) {
        let expected_state = response.create_state();

        game_state.with_untracked(|actual| {
            assert_eq!(actual.game_id.as_ref(), Some(&response.game_id));
            assert_eq!(actual.white_id, Some(response.white_player.uid));
            assert_eq!(actual.black_id, Some(response.black_player.uid));
            assert_eq!(actual.state, expected_state);
            assert_eq!(actual.move_info, MoveInfo::new());
            assert_eq!(actual.board_view, BoardView::Live);
            assert_eq!(actual.game_control_pending, pending_control);

            let stored_response = actual
                .game_response
                .as_ref()
                .expect("authoritative response is stored");
            assert_eq!(stored_response.uuid, response.uuid);
            assert_eq!(stored_response.game_id, response.game_id);
            assert_eq!(
                stored_response.current_player_id,
                response.current_player_id
            );
            assert_eq!(stored_response.history, response.history);
            assert_eq!(
                stored_response.game_control_history,
                response.game_control_history
            );
            assert_eq!(stored_response.finished, response.finished);
            assert_eq!(stored_response.game_status, response.game_status);
        });
    }

    #[test]
    fn show_moves_marks_selected_piece_as_board_piece() {
        with_store(|game_state| {
            let origin = Position::new(0, 0);
            game_state.state().update(|state| {
                state.game_status = GameStatus::InProgress;
                state.turn_color = Color::White;
                state.board.insert(origin, piece("wQ"), true);
                state
                    .board
                    .insert(origin.to(BoardDirection::E), piece("bQ"), true);
                state.board.insert(origin, piece("wB1"), true);
                state.board.last_moved = None;
            });

            game_state.show_moves(piece("wB1"), origin);

            game_state.move_info().with_untracked(|move_info| {
                assert_eq!(move_info.active, Some((piece("wB1"), PieceType::Board)));
                assert_eq!(move_info.current_position, Some(origin));
                assert!(!move_info.target_positions.is_empty());
                assert_eq!(move_info.reserve_position, None);
            });
        });
    }

    #[test]
    fn show_spawns_marks_selected_piece_as_reserve_piece() {
        with_store(|game_state| {
            let reserve_position = Position::new(0, 0);
            game_state.state().update(|state| {
                state.game_status = GameStatus::InProgress;
                state.turn_color = Color::White;
            });

            game_state.show_spawns(piece("wA1"), reserve_position);

            game_state.move_info().with_untracked(|move_info| {
                assert_eq!(move_info.active, Some((piece("wA1"), PieceType::Reserve)));
                assert_eq!(move_info.current_position, None);
                assert_eq!(move_info.reserve_position, Some(reserve_position));
                assert!(move_info
                    .target_positions
                    .contains(&Position::initial_spawn_position()));
            });
        });
    }

    #[test]
    fn reset_from_unfinished_response_replaces_dirty_game_and_keeps_latest_offer() {
        with_store(|game_state| {
            dirty_game_a(game_state);

            let mut response = game_response_with_valid_history("game-b");
            response
                .game_control_history
                .push((0, GameControl::DrawOffer(Color::White)));

            game_state.reset_from_response(&response);

            assert_authoritative_snapshot(
                game_state,
                &response,
                Some(GameControl::DrawOffer(Color::White)),
            );
        });
    }

    #[test]
    fn reset_from_finished_response_replaces_dirty_game_and_clears_pending_control() {
        with_store(|game_state| {
            dirty_game_a(game_state);

            let mut response = game_response_with_valid_history("game-b");
            response.finished = true;
            response.game_status = GameStatus::Adjudicated;
            response
                .game_control_history
                .push((0, GameControl::TakebackRequest(Color::Black)));

            game_state.reset_from_response(&response);

            assert_authoritative_snapshot(game_state, &response, None);
        });
    }
}
