use std::{ops::Deref, str::FromStr};

use crate::{
    common::{MoveInfo, PieceType},
    responses::GameResponse,
};
use hive_lib::{Color, GameControl, GameStatus, GameType, Piece, Position, State, Turn};
use leptos::{logging::log, prelude::*};
use reactive_stores::Store;
use shared_types::{GameId, GameSpeed, Takeback};
use uuid::Uuid;

use super::{
    analysis::AnalysisSignal,
    api_requests::ApiRequests,
    auth_context::AuthContext,
    ApiRequestsProvider,
};

#[derive(Clone, Copy)]
pub struct GameStateStore(pub Store<GameState>);

impl Deref for GameStateStore {
    type Target = Store<GameState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GameStateStore {
    pub fn player_ids(&self) -> Signal<(Option<Uuid>, Option<Uuid>)> {
        let white_id = self.white_id();
        let black_id = self.black_id();
        Signal::derive(move || (white_id.get(), black_id.get()))
    }

    pub fn game_status(&self) -> Signal<GameStatus> {
        let state = self.state();
        Signal::derive(move || state.with(|state| state.game_status.clone()))
    }

    pub fn is_last_turn_untracked(&self) -> bool {
        let state_turn = self.state().with_untracked(|state| state.turn);
        if state_turn == 0 {
            return true;
        }
        self.history_turn()
            .with_untracked(|history_turn| *history_turn == Some(state_turn - 1))
    }

    pub fn view_history_at_last_turn(&self) {
        let turn = self
            .state()
            .with_untracked(|state| state.turn.checked_sub(1));
        self.0.update(|s| {
            s.view = View::History;
            s.history_turn = turn;
        });
    }

    pub fn takeback_allowed(&self) -> bool {
        let color_allowed = |color: &Color, game_response: &GameResponse| {
            let rated = game_response.rated;
            let takeback = match color {
                Color::Black => &game_response.black_player.takeback,
                Color::White => &game_response.white_player.takeback,
            };
            takeback == &Takeback::Always || takeback == &Takeback::CasualOnly && !rated
        };
        self.game_response().with(|game_response| {
            game_response.as_ref().is_some_and(|game_response| {
                color_allowed(&Color::Black, game_response)
                    && color_allowed(&Color::White, game_response)
            })
        })
    }

    pub fn full_reset(&self) {
        let state = State::new(GameType::MLP, false);
        self.0.update(|s| {
            s.game_id = None;
            s.state = state;
            s.black_id = None;
            s.white_id = None;
            s.move_info.reset();
            s.history_turn = None;
            s.view = View::Game;
            s.game_control_pending = None;
        });
    }

    pub fn color_for_user_signal(&self, user_id: Signal<Option<Uuid>>) -> Signal<Option<Color>> {
        let white_id = self.white_id();
        let black_id = self.black_id();
        Signal::derive(move || {
            let white = white_id.get();
            let black = black_id.get();
            let current_user_id = user_id()?;
            if Some(current_user_id) == black {
                Some(Color::Black)
            } else if Some(current_user_id) == white {
                Some(Color::White)
            } else {
                None
            }
        })
    }

    pub fn user_is_player_signal(&self, user_id: Signal<Option<Uuid>>) -> Signal<bool> {
        let user_color = self.color_for_user_signal(user_id);
        Signal::derive(move || user_color().is_some())
    }

    pub fn undo_move(&self) {
        self.0.update(|s| {
            if let Some(turn) = s.history_turn {
                s.state.undo();
                if turn > 0 {
                    s.history_turn = Some(turn - 1);
                } else {
                    s.history_turn = None;
                }
            };
        })
    }

    pub fn set_game_status(&self, status: GameStatus) {
        self.state().update(|state| {
            state.game_status = status;
        })
    }

    pub fn set_pending_gc(&self, gc: GameControl) {
        self.game_control_pending().set(Some(gc))
    }

    pub fn clear_gc(&self) {
        self.game_control_pending().set(None)
    }

    pub fn send_game_control(&self, game_control: GameControl, user: Uuid) {
        let api = expect_context::<ApiRequestsProvider>().0;
        self.0.with_untracked(|gs| {
            if let Some(color) = gs.user_color(Some(user)) {
                if color != game_control.color() {
                    log!("This is a bug, you should only send GCs of your own color, user id color is {color} and gc color is {}", game_control.color());
                } else if let Some(ref game_id) = gs.game_id {
                    api.get().game_control(game_id.to_owned(), game_control);
                } else {
                    log!("This is a bug, there should be a game_id");
                }
            }
        })
    }

    pub fn set_state(&self, state: State, black_id: Uuid, white_id: Uuid) {
        let turn = if state.turn != 0 {
            Some(state.turn - 1)
        } else {
            None
        };
        self.0.update(|s| {
            s.move_info.reset();
            s.history_turn = turn;
            s.state = state;
            s.black_id = Some(black_id);
            s.white_id = Some(white_id);
        });
    }

    pub fn set_game_id(&self, game_id: GameId) {
        self.game_id().set(Some(game_id))
    }

    pub fn play_turn(&self, piece: Piece, position: Position) {
        self.state().update(|state| {
            if let Err(e) = state.play_turn_from_position(piece, position) {
                log!("Could not play turn: {} {} {}", piece, position, e);
            }
        })
    }

    pub fn reset(&self) {
        self.move_info().update(|move_info| move_info.reset())
    }

    pub fn move_active(&self, analysis: Option<AnalysisSignal>, api: ApiRequests) {
        self.0.update(|s| s.move_active(analysis, api))
    }

    pub fn is_move_allowed(&self, in_analysis: bool) -> bool {
        self.0.with_untracked(|gs| gs.is_move_allowed(in_analysis))
    }

    pub fn show_moves(&self, piece: Piece, position: Position) {
        let target_positions = self.state().with_untracked(|state| {
            let moves = state.board.moves(state.turn_color);
            moves.get(&(piece, position)).cloned()
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
            let turn_color = state.turn_color;
            let board = &state.board;
            let game_type = state.game_type;
            let target_positions = board
                .spawnable_positions(turn_color)
                .collect::<Vec<Position>>();
            let active = board
                .reserve(turn_color, game_type)
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
        self.move_info()
            .update(|move_info| move_info.target_position = Some(position))
    }

    pub fn show_history_turn(&self, turn: usize) {
        self.history_turn().set(Some(turn))
    }

    pub fn first_history_turn(&self) {
        self.0.update(|s| s.first_history_turn())
    }

    pub fn next_history_turn(&self) {
        self.0.update(|s| {
            s.next_history_turn();
            if let Some(turn) = s.history_turn {
                if s.state.history.move_is_pass(turn) {
                    s.next_history_turn()
                }
            }
        });
    }

    pub fn previous_history_turn(&self) {
        self.0.update(|s| {
            s.previous_history_turn();
            if let Some(turn) = s.history_turn {
                if s.state.history.move_is_pass(turn) {
                    s.previous_history_turn()
                }
            }
        });
    }

    pub fn view_game(&self) {
        self.0.update(|s| s.view_game())
    }

    pub fn view_history(&self) {
        self.view().set(View::History)
    }

    pub fn set_game_response(&self, game_response: GameResponse) {
        self.game_response().set(Some(game_response));
    }

    pub fn is_finished(&self) -> Memo<bool> {
        let state = self.state();
        let game_status_finished = Signal::derive(move || {
            state.with(|state| {
                matches!(
                    state.game_status,
                    GameStatus::Finished(_) | GameStatus::Adjudicated
                )
            })
        });
        let game_response = self.game_response();
        let game_response_finished = Signal::derive(move || {
            game_response.with(|game_response| {
                game_response
                    .as_ref()
                    .is_some_and(|game_response| game_response.finished)
            })
        });
        Memo::new(move |_| game_status_finished() || game_response_finished())
    }

    pub fn is_last_turn_as_signal(&self) -> Signal<bool> {
        let state = self.state();
        let history_turn = self.history_turn();
        Signal::derive(move || {
            let state_turn = state.with(|state| state.turn);
            if state_turn == 0 {
                return true;
            }
            history_turn.with(|history_turn| *history_turn == Some(state_turn - 1))
        })
    }

    pub fn is_first_turn_as_signal(&self) -> Signal<bool> {
        let history_turn = self.history_turn();
        Signal::derive(move || {
            history_turn.with(|history_turn| history_turn.is_none() || *history_turn == Some(0))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    History,
    Game,
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
    // the turn we want to display the history at
    pub history_turn: Option<usize>,
    // show history or reserve
    pub view: View,
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
    // TODO get the state from URL/game_id via a call
    pub fn new() -> Self {
        let state = State::new(GameType::MLP, false);
        Self {
            game_id: None,
            state,
            black_id: None,
            white_id: None,
            move_info: MoveInfo::new(),
            history_turn: None,
            view: View::Game,
            game_control_pending: None,
            game_response: None,
        }
    }

    pub fn new_with_game_type(game_type: GameType) -> Self {
        let state = State::new(game_type, false);
        Self {
            game_id: None,
            state,
            black_id: None,
            white_id: None,
            move_info: MoveInfo::new(),
            history_turn: None,
            view: View::Game,
            game_control_pending: None,
            game_response: None,
        }
    }

    // Still needed because send_game_control uses it, maybe this should be moved out of the gamestate?
    pub fn user_color(&self, user_id: Option<Uuid>) -> Option<Color> {
        user_id?;
        if user_id == self.black_id {
            return Some(Color::Black);
        }
        if user_id == self.white_id {
            return Some(Color::White);
        }
        None
    }

    pub fn uid_is_player(&self, user_id: Option<Uuid>) -> bool {
        user_id.is_some() && (user_id == self.white_id || user_id == self.black_id)
    }

    pub fn play_turn(&mut self, piece: Piece, position: Position) {
        if let Err(e) = self.state.play_turn_from_position(piece, position) {
            log!("Could not play turn: {} {} {}", piece, position, e);
        }
    }

    pub fn set_target(&mut self, position: Position) {
        self.move_info.target_position = Some(position);
    }

    pub fn is_move_allowed(&self, analysis: bool) -> bool {
        let auth_context = expect_context::<AuthContext>();
        let user = auth_context.user;
        if analysis {
            return true;
        }
        if matches!(
            self.state.game_status,
            GameStatus::Finished(_) | GameStatus::Adjudicated
        ) {
            return false;
        }
        user.with_untracked(|a| {
            a.as_ref().is_some_and(|user| {
                let turn = self.state.turn;
                let black_id = self.black_id;
                let white_id = self.white_id;
                if turn.is_multiple_of(2) {
                    white_id.is_some_and(|white| white == user.id)
                } else {
                    black_id.is_some_and(|black| black == user.id)
                }
            })
        })
    }

    pub fn move_active(&mut self, analysis: Option<AnalysisSignal>, api: ApiRequests) {
        if let (Some((active, _)), Some(position)) =
            (self.move_info.active, self.move_info.target_position)
        {
            if let Err(e) = self.state.play_turn_from_position(active, position) {
                log!("Could not play turn: {} {} {}", active, position, e);
            } else if let Some(analysis) = analysis {
                analysis.0.update(|analysis| {
                    let moves = self.state.history.moves.clone();
                    let hashes = self.state.hashes.clone();
                    let last_index = moves.len() - 1;
                    if moves[last_index].0 == "pass" {
                        //if move is pass, add prev move
                        analysis.add_node(moves[last_index - 1].clone(), hashes[last_index - 1]);
                    }
                    analysis.add_node(moves[last_index].clone(), hashes[last_index]);
                    self.move_info.reset();
                });
            } else if let Some(ref game_id) = self.game_id {
                let turn = Turn::Move(active, position);
                api.turn(game_id.to_owned(), turn);
                self.move_info.reset();
                self.history_turn = Some(self.state.turn - 1);
            }
        }
    }

    // TODO refactor to not take a position, the position and piece are in self already
    pub fn show_moves(&mut self, piece: Piece, position: Position) {
        self.move_info.reset();
        self.move_info.current_position = Some(position);
        let moves = self.state.board.moves(self.state.turn_color);
        if let Some(positions) = moves.get(&(piece, position)) {
            positions.clone_into(&mut self.move_info.target_positions);
            self.move_info.active = Some((piece, PieceType::Board));
        }
    }

    pub fn show_spawns(&mut self, piece: Piece, position: Position) {
        self.move_info.reset();
        let turn_color = self.state.turn_color;
        let board = &self.state.board;
        let game_type = self.state.game_type;

        self.move_info.target_positions = board
            .spawnable_positions(turn_color)
            .collect::<Vec<Position>>();
        let reserve = board.reserve(turn_color, game_type);
        if let Some(pieces) = reserve.get(&piece.bug()) {
            if let Some(piece) = pieces.first() {
                if let Ok(piece) = Piece::from_str(piece) {
                    self.move_info.active = Some((piece, PieceType::Reserve));
                    self.move_info.reserve_position = Some(position);
                }
            }
        }
    }

    pub fn show_history_turn(&mut self, turn: usize) {
        self.history_turn = Some(turn);
    }

    pub fn view_history(&mut self) {
        self.view = View::History;
    }

    //TODO: is this still useful for play and analysis where gamestate is untracked for the callback?
    pub fn is_last_turn(&self) -> bool {
        if self.state.turn == 0 {
            return true;
        }
        self.history_turn == Some(self.state.turn - 1)
    }

    pub fn view_game(&mut self) {
        self.view = View::Game;
        if self.state.turn > 0 {
            self.history_turn = Some(self.state.turn - 1);
        }
    }

    pub fn next_history_turn(&mut self) {
        self.view = View::History;
        if let Some(turn) = self.history_turn {
            self.history_turn = Some(std::cmp::min(turn + 1, self.state.turn - 1));
        } else if self.state.turn > 0 {
            self.history_turn = Some(0);
        }
    }

    pub fn previous_history_turn(&mut self) {
        self.view = View::History;
        if let Some(turn) = self.history_turn {
            self.history_turn = Some(turn.saturating_sub(1));
            if turn == 0 {
                self.history_turn = None;
            }
        }
    }

    pub fn first_history_turn(&mut self) {
        self.view = View::History;
        if self.state.turn > 0 {
            self.history_turn = Some(0)
        } else {
            self.history_turn = None;
        }
    }

    pub fn get_game_speed(&self) -> Option<GameSpeed> {
        self.game_response.as_ref().map(|gr| gr.speed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_lib::Direction as BoardDirection;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    #[test]
    fn show_moves_marks_selected_piece_as_board_piece() {
        let origin = Position::new(0, 0);
        let mut game_state = GameState::new_with_game_type(GameType::MLP);
        game_state.state.game_status = GameStatus::InProgress;
        game_state.state.turn_color = Color::White;
        game_state.state.board.insert(origin, piece("wQ"), true);
        game_state
            .state
            .board
            .insert(origin.to(BoardDirection::E), piece("bQ"), true);
        game_state.state.board.insert(origin, piece("wB1"), true);
        game_state.state.board.last_moved = None;

        assert!(game_state
            .state
            .board
            .moves(Color::White)
            .contains_key(&(piece("wB1"), origin)));

        game_state.show_moves(piece("wB1"), origin);

        assert_eq!(
            game_state.move_info.active,
            Some((piece("wB1"), PieceType::Board))
        );
        assert_eq!(game_state.move_info.current_position, Some(origin));
        assert!(!game_state.move_info.target_positions.is_empty());
        assert_eq!(game_state.move_info.reserve_position, None);
    }

    #[test]
    fn show_spawns_marks_selected_piece_as_reserve_piece() {
        let reserve_position = Position::new(0, 0);
        let mut game_state = GameState::new_with_game_type(GameType::Base);
        game_state.state.game_status = GameStatus::InProgress;
        game_state.state.turn_color = Color::White;

        game_state.show_spawns(piece("wA1"), reserve_position);

        assert_eq!(
            game_state.move_info.active,
            Some((piece("wA1"), PieceType::Reserve))
        );
        assert_eq!(game_state.move_info.current_position, None);
        assert_eq!(
            game_state.move_info.reserve_position,
            Some(reserve_position)
        );
        assert!(game_state
            .move_info
            .target_positions
            .contains(&Position::initial_spawn_position()));
    }

    #[test]
    fn user_color_returns_none_when_user_or_players_are_unloaded() {
        let user_id = Uuid::new_v4();
        let mut game_state = GameState::new();

        assert_eq!(game_state.user_color(None), None);
        assert_eq!(game_state.user_color(Some(user_id)), None);

        game_state.black_id = Some(user_id);

        assert_eq!(game_state.user_color(None), None);
        assert_eq!(game_state.user_color(Some(user_id)), Some(Color::Black));
    }
}
