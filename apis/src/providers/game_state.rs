use crate::providers::api_requests::ApiRequests;
use hive_lib::color::Color;
use hive_lib::game_control::GameControl;
use hive_lib::game_status::GameStatus;
use hive_lib::{game_type::GameType, piece::Piece, position::Position, state::State, turn::Turn};
use leptos::logging::log;
use leptos::*;
use uuid::Uuid;

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

    pub fn user_color(&self, user_id: Uuid) -> Option<Color> {
        self.signal.get_untracked().user_color(user_id)
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
        log!("Sending game_control: {game_control}");
        self.signal
            .get_untracked()
            .send_game_control(game_control, user)
    }

    pub fn join(&self) {
        log!("Joined game");
        self.signal.get_untracked().join()
    }

    pub fn set_state(&mut self, state: State, black_id: Uuid, white_id: Uuid) {
        self.reset();
        self.signal.update(|s| {
            s.state = state;
            s.black_id = Some(black_id);
            s.white_id = Some(white_id);
        })
    }

    pub fn set_game_id(&mut self, game_id: StoredValue<String>) {
        log!("game id is {}", game_id());
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

    pub fn spawn_active(&mut self) {
        self.signal.update(|s| s.spawn_active())
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
        self.signal.update(|s| s.next_history_turn())
    }

    pub fn previous_history_turn(&mut self) {
        self.signal.update(|s| s.previous_history_turn())
    }

    pub fn view_game(&mut self) {
        self.signal.update(|s| s.view_game())
    }

    pub fn view_history(&mut self) {
        self.signal.update(|s| s.view_history())
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
    pub game_id: Option<StoredValue<String>>,
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
        let game_id = self.game_id.expect("Game_id in gamestate")();
        if let Some(color) = self.user_color(user_id) {
            if color != game_control.color() {
                log!("This is a bug, you should only send GCs of your own color");
            } else {
                ApiRequests::new().game_control(game_id, game_control)
            }
        }
    }

    pub fn join(&mut self) {
        let game_id = self.game_id.expect("Game_id in gamestate")();
        ApiRequests::new().join(game_id)
    }

    pub fn move_active(&mut self) {
        if let (Some(active), Some(position)) = (self.active, self.target_position) {
            if let Err(e) = self.state.play_turn_from_position(active, position) {
                log!("Could not play turn: {} {} {}", active, position, e);
            } else {
                let game_id = self.game_id.expect("Game_id in gamestate")();
                let turn = Turn::Move(active, position);
                ApiRequests::new().turn(game_id, turn)
            }
        }
        self.reset();
        self.game_control_pending = None;
        self.history_turn = Some(self.state.turn - 1)
    }

    pub fn spawn_active(&mut self) {
        if let (Some(active), Some(position)) = (self.active, self.target_position) {
            if let Err(e) = self.state.play_turn_from_position(active, position) {
                log!("Could not play turn: {} {} {}", active, position, e);
            } else {
                let game_id = self.game_id.expect("Game_id in gamestate")();
                let turn = Turn::Spawn(active, position);
                ApiRequests::new().turn(game_id, turn)
            }
        }
        self.reset();
        self.history_turn = Some(self.state.turn - 1)
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
        log!("showing moves");
        if let Some(positions) = moves.get(&(piece, position)) {
            self.target_positions = positions.to_owned();
            log!("{:?}", piece);
            self.active = Some(piece);
        }
    }

    pub fn show_spawns(&mut self, piece: Piece, position: Position) {
        self.reset();
        self.target_positions = self
            .state
            .board
            .spawnable_positions(self.state.turn_color)
            .collect::<Vec<Position>>();
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
