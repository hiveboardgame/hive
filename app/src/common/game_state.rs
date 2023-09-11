use hive_lib::{game_type::GameType, piece::Piece, position::Position, state::State};
use leptos::*;

#[derive(Clone, Debug, Copy)]
pub struct GameStateSignal {
    pub signal: RwSignal<GameState>,
}

impl GameStateSignal {
    pub fn new(cx: Scope) -> Self {
        Self {
            signal: create_rw_signal(cx, GameState::new()),
        }
    }

    pub fn reset(&mut self) {
        self.signal.update(|s| s.reset())
    }

    pub fn spawn_active_piece(&mut self) {
        self.signal.update(|s| s.spawn_active_piece())
    }

    pub fn show_moves(&mut self, piece: Piece, position: Position) {
        self.signal.update(|s| s.show_moves(piece, position))
    }

    pub fn show_spawns(&mut self, piece: Piece) {
        self.signal.update(|s| s.show_spawns(piece))
    }
    pub fn set_target(&mut self, position: Position) {
        self.signal.update(|s| s.set_target(position))
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub state: State,
    pub target_positions: Vec<Position>,
    pub active: Option<Piece>,
    pub position: Option<Position>,
}

impl GameState {
    // TODO get the state from URL/game_id via a call
    pub fn new() -> Self {
        let state = State::new(GameType::MLP, true);
        Self {
            state,
            target_positions: vec![],
            active: None,
            position: None,
        }
    }

    pub fn set_target(&mut self, position: Position) {
        self.position = Some(position);
        self.target_positions.clear();
    }

    pub fn reset(&mut self) {
        self.target_positions.clear();
        self.active = None;
        self.position = None;
    }

    pub fn spawn_active_piece(&mut self) {
        if let (Some(active), Some(position)) = (self.active, self.position) {
            match self.state.play_turn_from_position(active, position) {
                Err(e) => log!("Could not play turn: {} {} {}", active, position, e),
                _ => {}
            };
        }
        self.reset()
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
        let moves = self.state.board.moves(self.state.turn_color);
        if let Some(positions) = moves.get(&(piece, position)) {
            self.target_positions = positions.to_owned();
            self.active = Some(piece);
        }
    }

    pub fn show_spawns(&mut self, piece: Piece) {
        self.reset();
        self.target_positions = self
            .state
            .board
            .spawnable_positions(self.state.turn_color)
            .collect::<Vec<Position>>();
        self.active = Some(piece);
    }
}
