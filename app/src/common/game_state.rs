use hive_lib::{history::History, piece::Piece, position::Position, state::State, game_type::GameType};
use leptos::*;

#[derive(Clone, Debug, Copy)]
pub struct GameState {
    pub state: RwSignal<State>,
    pub target_positions: RwSignal<Vec<Position>>,
    pub active: RwSignal<Option<Piece>>,
    pub position: RwSignal<Option<Position>>,
}

impl GameState {
    // TODO get the state from URL/game_id via a call
    pub fn new(cx: Scope) -> Self {
        // let history = History::from_filepath("engine/test_pgns/valid/descend.pgn").unwrap();
        // let state = State::new_from_history(&history).unwrap();
        let state = State::new(GameType::MLP, true);
        Self {
            state: create_rw_signal(cx, state),
            target_positions: create_rw_signal(cx, vec![]),
            active: create_rw_signal(cx, None),
            position: create_rw_signal(cx, None),
        }
    }

    pub fn reset(&mut self) {
        self.target_positions.set(vec![]);
        self.active.set(None);
        self.position.set(None);
    }

    pub fn spawn_active_piece(&mut self) {
        if let (Some(active), Some(position)) = (self.active.get(), self.position.get()) {
            if !self.state.get().play_turn(active, position).is_ok() {
                log!("Could not play turn: {} {}", active, position);
            };
        }
        self.reset()
    }

    // TODO refactor to not take a position, the position and piece are in self already
    pub fn show_moves(&mut self, piece: Piece, position: Position) {
        if let Some(already) = self.active.get() {
            if piece == already {
                self.reset();
                return;
            }
        }
        self.reset();
        let moves = self.state.get().board.moves(self.state.get().turn_color);
        if let Some(positions) = moves.get(&(piece, position)) {
            self.target_positions.set(positions.to_owned());
            self.active.set(Some(piece));
        }
    }

    pub fn show_spawns(&mut self, piece: Piece) {
        self.reset();
        self.target_positions.set(
            self.state
                .get()
                .board
                .spawnable_positions(self.state.get().turn_color)
                .collect::<Vec<Position>>(),
        );
        self.active.set(Some(piece));
    }
}
