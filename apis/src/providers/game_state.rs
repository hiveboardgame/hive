use std::str::FromStr;

use crate::common::MoveInfo;
use crate::providers::api_requests::ApiRequests;
use crate::responses::GameResponse;
use hive_lib::{Color, GameControl, GameStatus, GameType, Piece, Position, State, Turn};
use leptos::logging::log;
use leptos::prelude::*;
use shared_types::{GameId, GameSpeed, Takeback};
use uuid::Uuid;

use super::auth_context::AuthContext;

#[derive(Clone, Debug, Copy)]
pub struct GameStateSignal {
    pub signal: RwSignal<GameState>,
    pub loaded: RwSignal<bool>,
}

impl Default for GameStateSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateSignal {
    pub fn new() -> Self {
        Self {
            signal: RwSignal::new(GameState::new()),
            loaded: RwSignal::new(false),
        }
    }

    pub fn full_reset(&mut self) {
        let state = State::new(GameType::MLP, false);
        self.signal.update(|s| {
            s.game_id = None;
            s.state = state;
            s.black_id = None;
            s.white_id = None;
            s.move_info.reset();
            s.history_turn = None;
            s.view = View::Game;
            s.game_control_pending = None;
        })
    }

    pub fn do_analysis(&mut self) {
        self.signal.update(|s| {
            s.view = View::Game;
            s.game_id = None;
            s.state.game_status = GameStatus::InProgress;
            s.black_id = None;
            s.white_id = None;
        });
    }

    // No longer access the whole signal when getting user_color
    pub fn user_color_as_signal(&self, user_id: Signal<Option<Uuid>>) -> Signal<Option<Color>> {
        create_read_slice(self.signal, move |gamestate| {
            match (gamestate.white_id, gamestate.black_id) {
                (Some(w), Some(b)) => {
                    if user_id() == Some(b) {
                        Some(Color::Black)
                    } else if user_id() == Some(w) {
                        return Some(Color::White);
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    pub fn undo_move(&mut self) {
        self.signal.update(|s| {
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
        self.signal.update(|s| {
            s.state.game_status = status;
        })
    }

    pub fn set_pending_gc(&self, gc: GameControl) {
        self.signal.update(|s| {
            s.game_control_pending = Some(gc);
        })
    }

    pub fn clear_gc(&self) {
        self.signal.update(|s| {
            s.game_control_pending = None;
        })
    }

    pub fn send_game_control(&self, game_control: GameControl, user: Uuid) {
        self.signal
            .get_untracked()
            .send_game_control(game_control, user)
    }

    pub fn set_state(&mut self, state: State, black_id: Uuid, white_id: Uuid) {
        self.reset();
        let turn = if state.turn != 0 {
            Some(state.turn - 1)
        } else {
            None
        };
        self.signal.update(|s| {
            s.history_turn = turn;
            s.state = state;
            s.black_id = Some(black_id);
            s.white_id = Some(white_id);
        })
    }

    pub fn set_game_id(&mut self, game_id: GameId) {
        self.signal.update_untracked(|s| s.game_id = Some(game_id))
    }

    pub fn play_turn(&mut self, piece: Piece, position: Position) {
        self.signal.update(|s| {
            s.play_turn(piece, position);
            s.move_info.reset()
        })
    }

    pub fn reset(&mut self) {
        self.signal.update(|s| s.move_info.reset())
    }

    pub fn move_active(&mut self) {
        self.signal.update(|s| s.move_active())
    }

    pub fn is_move_allowed(&self) -> bool {
        self.signal.get_untracked().is_move_allowed()
    }

    pub fn show_moves(&mut self, piece: Piece, position: Position) {
        self.signal.update(|s| s.show_moves(piece, position))
    }

    pub fn show_spawns(&mut self, piece: Piece, position: Position) {
        self.signal.update(|s| s.show_spawns(piece, position))
    }

    pub fn set_target(&mut self, position: Position) {
        self.signal.update(|s| s.set_target(position))
    }

    pub fn show_history_turn(&self, turn: usize) {
        self.signal.update(|s| s.show_history_turn(turn))
    }

    pub fn first_history_turn(&mut self) {
        self.signal.update(|s| s.first_history_turn())
    }

    pub fn next_history_turn(&mut self) {
        self.signal.update(|s| {
            s.next_history_turn();
            if let Some(turn) = s.history_turn {
                if s.state.history.move_is_pass(turn) {
                    s.next_history_turn()
                }
            }
        });
    }

    pub fn previous_history_turn(&mut self) {
        self.signal.update(|s| {
            s.previous_history_turn();
            if let Some(turn) = s.history_turn {
                if s.state.history.move_is_pass(turn) {
                    s.previous_history_turn()
                }
            }
        });
    }

    pub fn view_game(&mut self) {
        self.signal.update(|s| s.view_game())
    }

    pub fn view_history(&mut self) {
        self.signal.update(|s| s.view_history())
    }

    pub fn set_game_response(&mut self, game_response: GameResponse) {
        self.signal
            .update(|s| s.game_response = Some(game_response));
    }

    pub fn is_finished(&self) -> Memo<bool> {
        let game_status_finished = create_read_slice(self.signal, |game_state| {
            matches!(game_state.state.game_status, GameStatus::Finished(_))
        });
        let game_response_finished = create_read_slice(self.signal, |game_state| {
            game_state
                .game_response
                .as_ref()
                .is_some_and(|gr| gr.finished)
        });
        Memo::new(move |_| game_status_finished() || game_response_finished())
    }

    pub fn is_last_turn_as_signal(&self) -> Signal<bool> {
        create_read_slice(self.signal, |gs| {
            if gs.state.turn == 0 {
                true
            } else {
                gs.history_turn == Some(gs.state.turn - 1)
            }
        })
    }

    pub fn is_first_turn_as_signal(&self) -> Signal<bool> {
        create_read_slice(self.signal, |gs| {
            gs.history_turn.is_none() || gs.history_turn == Some(0)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    History,
    Game,
}

#[derive(Clone, Debug)]
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
    fn user_color(&self, user_id: Uuid) -> Option<Color> {
        if Some(user_id) == self.black_id {
            return Some(Color::Black);
        }
        if Some(user_id) == self.white_id {
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

    pub fn send_game_control(&mut self, game_control: GameControl, user_id: Uuid) {
        if let Some(color) = self.user_color(user_id) {
            if color != game_control.color() {
                log!("This is a bug, you should only send GCs of your own color, user id color is {color} and gc color is {}", game_control.color());
            } else if let Some(ref game_id) = self.game_id {
                ApiRequests::new().game_control(game_id.to_owned(), game_control);
            } else {
                log!("This is a bug, there should be a game_id");
            }
        }
    }

    pub fn is_move_allowed(&self) -> bool {
        let auth_context = expect_context::<AuthContext>();

        let user = move || match auth_context.user.get() {
            Some(Ok(Some(user))) => Some(user),
            _ => None,
        };
        if matches!(self.state.game_status, GameStatus::Finished(_)) {
            return false;
        }
        user().is_some_and(|user| {
            let turn = self.state.turn;
            let black_id = self.black_id;
            let white_id = self.white_id;
            if turn % 2 == 0 {
                white_id.is_some_and(|white| white == user.id)
            } else {
                black_id.is_some_and(|black| black == user.id)
            }
        })
    }

    pub fn move_active(&mut self) {
        //log!("Moved active!");
        if let (Some(active), Some(position)) =
            (self.move_info.active, self.move_info.target_position)
        {
            if let Err(e) = self.state.play_turn_from_position(active, position) {
                log!("Could not play turn: {} {} {}", active, position, e);
            } else if let Some(ref game_id) = self.game_id {
                let turn = Turn::Move(active, position);
                ApiRequests::new().turn(game_id.to_owned(), turn);
                self.move_info.reset();
                self.history_turn = Some(self.state.turn - 1);
            } else {
                log!("We should be in analysis");
                self.move_info.reset();
                self.history_turn = Some(self.state.turn - 1);
            }
        }
    }

    // TODO refactor to not take a position, the position and piece are in self already
    pub fn show_moves(&mut self, piece: Piece, position: Position) {
        if let Some(already) = self.move_info.active {
            if piece == already {
                self.move_info.reset();
                return;
            }
        }
        self.move_info.reset();
        self.move_info.current_position = Some(position);
        let moves = self.state.board.moves(self.state.turn_color);
        if let Some(positions) = moves.get(&(piece, position)) {
            positions.clone_into(&mut self.move_info.target_positions);
            self.move_info.active = Some(piece);
        }
    }

    pub fn show_spawns(&mut self, piece: Piece, position: Position) {
        self.move_info.reset();
        self.move_info.target_positions = self
            .state
            .board
            .spawnable_positions(self.state.turn_color)
            .collect::<Vec<Position>>();
        let reserve = self
            .state
            .board
            .reserve(self.state.turn_color, self.state.game_type);
        if let Some(pieces) = reserve.get(&piece.bug()) {
            if let Some(piece) = pieces.first() {
                if let Ok(piece) = Piece::from_str(piece) {
                    self.move_info.active = Some(piece);
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
        self.game_response.as_ref().map(|gr| gr.speed.clone())
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
        if let Some(game_response) = self.game_response.as_ref() {
            let white = color_allowed(&Color::Black, game_response);
            let black = color_allowed(&Color::White, game_response);
            white && black
        } else {
            false
        }
    }
}

pub fn provide_game_state() {
    provide_context(GameStateSignal::new())
}
