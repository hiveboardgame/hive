use crate::providers::api_requests::ApiRequests;
use crate::responses::game::GameResponse;
use hive_lib::color::Color;
use hive_lib::game_control::GameControl;
use hive_lib::game_status::GameStatus;
use hive_lib::{game_type::GameType, piece::Piece, position::Position, state::State, turn::Turn};
use leptos::logging::log;
use leptos::*;
use uuid::Uuid;

use super::auth_context::AuthContext;

#[derive(Clone, Debug, Copy)]
pub struct GameStateSignal {
    pub signal: RwSignal<GameState>,
}

impl Default for GameStateSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(GameState::new()),
        }
    }

    // TODO: fix this
    pub fn full_reset(&mut self) {
        let state = State::new(GameType::MLP, false);
        self.signal.update(|s| {
            s.game_id = None;
            s.state = state;
            s.black_id = None;
            s.white_id = None;
            s.target_positions = vec![];
            s.active = None;
            s.target_position = None;
            s.current_position = None;
            s.reserve_position = None;
            s.history_turn = None;
            s.view = View::Game;
            s.game_control_pending = None;
        })
    }

    pub fn do_analysis(&mut self) {
        self.signal.update(|s| {
            s.game_id = None;
            s.state.game_status = GameStatus::InProgress;
            s.black_id = None;
            s.white_id = None;
        });
    }

    pub fn user_color(&self, user_id: Uuid) -> Option<Color> {
        self.signal.get().user_color(user_id)
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

    pub fn set_game_id(&mut self, game_id: String) {
        self.signal.update_untracked(|s| s.game_id = Some(game_id))
    }

    pub fn play_turn(&mut self, piece: Piece, position: Position) {
        self.signal.update(|s| {
            s.play_turn(piece, position);
            s.reset()
        })
    }

    pub fn reset(&mut self) {
        self.signal.update(|s| s.reset())
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

    pub fn show_history_turn(&mut self, turn: usize) {
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
            .update(|s| s.game_response = Some(game_response))
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
    pub game_id: Option<String>,
    // the gamestate
    pub state: State,
    pub black_id: Option<Uuid>,
    pub white_id: Option<Uuid>,
    // possible destinations of selected piece
    pub target_positions: Vec<Position>,
    // the piece (either from reserve or board) that has been clicked last
    pub active: Option<Piece>,
    // the position of the piece that has been clicked last
    pub current_position: Option<Position>,
    // the position of the target that got clicked last
    pub target_position: Option<Position>,
    // the position of the reserve piece that got clicked last
    pub reserve_position: Option<Position>,
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
            target_positions: vec![],
            active: None,
            target_position: None,
            current_position: None,
            reserve_position: None,
            history_turn: None,
            view: View::Game,
            game_control_pending: None,
            game_response: None,
        }
    }

    pub fn user_color(&self, user_id: Uuid) -> Option<Color> {
        if Some(user_id) == self.black_id {
            return Some(Color::Black);
        }
        if Some(user_id) == self.white_id {
            return Some(Color::White);
        }
        None
    }

    pub fn play_turn(&mut self, piece: Piece, position: Position) {
        if let Err(e) = self.state.play_turn_from_position(piece, position) {
            log!("Could not play turn: {} {} {}", piece, position, e);
        }
    }

    pub fn set_target(&mut self, position: Position) {
        self.target_position = Some(position);
    }

    pub fn reset(&mut self) {
        self.target_positions.clear();
        self.active = None;
        self.target_position = None;
        self.current_position = None;
        self.reserve_position = None;
    }

    pub fn send_game_control(&mut self, game_control: GameControl, user_id: Uuid) {
        if let Some(color) = self.user_color(user_id) {
            if color != game_control.color() {
                log!("This is a bug, you should only send GCs of your own color");
            } else if let Some(ref game_id) = self.game_id {
                ApiRequests::new().game_control(game_id.to_owned(), game_control);
            } else {
                log!("This is a bug, there should be a game_id");
            }
        }
    }

    pub fn is_move_allowed(&self) -> bool {
        let auth_context = expect_context::<AuthContext>();

        let user = move || match (auth_context.user)() {
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
        log!("Moved active!");
        if let (Some(active), Some(position)) = (self.active, self.target_position) {
            if let Err(e) = self.state.play_turn_from_position(active, position) {
                log!("Could not play turn: {} {} {}", active, position, e);
            } else if let Some(ref game_id) = self.game_id {
                let turn = Turn::Move(active, position);
                ApiRequests::new().turn(game_id.to_owned(), turn);
                self.reset();
                self.history_turn = Some(self.state.turn - 1);
            } else {
                log!("We should be in analysis");
                self.reset();
                self.history_turn = Some(self.state.turn - 1);
            }
        }
    }

    // TODO refactor to not take a position, the position and piece are in self already
    pub fn show_moves(&mut self, piece: Piece, position: Position) {
        if let Some(already) = self.active {
            if piece == already {
                self.reset();
                return;
            }
        }
        self.reset();
        self.current_position = Some(position);
        let moves = self.state.board.moves(self.state.turn_color);
        if let Some(positions) = moves.get(&(piece, position)) {
            self.target_positions = positions.to_owned();
            self.active = Some(piece);
        }
    }

    pub fn show_spawns(&mut self, piece: Piece, position: Position) {
        self.reset();
        if self.state.turn == 1 {
            self.target_positions = vec![Position::initial_spawn_black()];
        } else {
            self.target_positions = self
                .state
                .board
                .spawnable_positions(self.state.turn_color)
                .collect::<Vec<Position>>();
        }
        self.active = Some(piece);
        self.reserve_position = Some(position);
    }

    pub fn show_history_turn(&mut self, turn: usize) {
        self.history_turn = Some(turn);
    }

    pub fn view_history(&mut self) {
        self.view = View::History;
        if self.state.turn > 0 {
            self.history_turn = Some(self.state.turn - 1);
        }
    }

    pub fn is_last_turn(&self) -> bool {
        if self.state.turn == 0 {
            return true;
        }
        self.history_turn == Some(self.state.turn - 1)
    }

    pub fn view_game(&mut self) {
        self.view = View::Game;
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
}

pub fn provide_game_state() {
    provide_context(GameStateSignal::new())
}
