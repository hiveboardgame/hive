use hudsoni::{
    Board,
    Bug,
    Color,
    GameResult,
    GameType,
    LeafUnmake,
    Piece,
    Position,
    State,
    Unmake,
};

use crate::tt::{side_key, square_key, stunned_key, turn_key};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(Piece, Position, Position),
    Place(Piece, Position),
    Pass,
}

pub struct Reversal {
    engine: Option<Unmake>,
    prev_turn: usize,
    prev_turn_color: Color,
    prev_last_moved: Option<(Piece, Position)>,
    prev_last_move: (Option<Position>, Option<Position>),
    prev_stunned: Option<Piece>,
    prev_hash: u64,
}

pub(crate) struct LeafReversal {
    engine: Option<LeafUnmake>,
    prev_turn_color: Color,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub board: Board,
    pub turn: usize,
    pub turn_color: Color,
    pub game_type: GameType,
    pub tournament: bool,
    pub hash: u64,
}

impl Game {
    pub fn from_state(state: &State) -> Self {
        let mut game = Self {
            board: state.board.clone(),
            turn: state.turn,
            turn_color: state.turn_color,
            game_type: state.game_type,
            tournament: state.tournament,
            hash: 0,
        };
        game.hash = game.compute_hash();
        game
    }

    fn square_term(&self, position: Position) -> u64 {
        let simple = self.board.board.get(position).simple();
        if simple == u32::MAX {
            0
        } else {
            square_key(position, simple)
        }
    }

    fn compute_hash(&self) -> u64 {
        let mut hash = 0;
        for offset in 0..48 {
            let piece = self.board.offset_to_piece(offset);
            if let Some(position) = self.board.position_of_piece(piece) {
                if self.board.is_bottom_piece(piece, position) {
                    hash ^= self.square_term(position);
                }
            }
        }
        if self.turn_color == Color::White {
            hash ^= side_key();
        }
        hash ^= turn_key(self.turn);
        if let Some(stunned) = self.board.stunned {
            hash ^= stunned_key(self.board.piece_to_offset(stunned));
        }
        hash
    }

    pub fn result(&self) -> GameResult {
        self.board.game_result()
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self.board.game_result(), GameResult::Unknown)
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        self.legal_actions_into(&mut actions);
        actions
    }

    pub fn legal_actions_into(&self, buf: &mut Vec<Action>) {
        buf.clear();
        if self.is_terminal() {
            return;
        }
        let color = self.turn_color;
        self.board.for_each_move(color, |piece, from, to| {
            buf.push(Action::Move(piece, from, to))
        });

        let must_place_queen = self.board.queen_required(self.turn, color);
        let queen_banned = self.tournament && self.turn < 2;
        let mut placeable: [Option<Piece>; 8] = [None; 8];
        let mut count = 0;
        self.board
            .for_each_placeable_piece(color, self.game_type, |piece| {
                if piece.bug() == Bug::Queen && queen_banned {
                    return;
                }
                if must_place_queen && piece.bug() != Bug::Queen {
                    return;
                }
                placeable[count] = Some(piece);
                count += 1;
            });
        if count > 0 {
            for to in self.board.spawnable_positions(color) {
                for piece in placeable.iter().take(count).flatten() {
                    buf.push(Action::Place(*piece, to));
                }
            }
        }

        if buf.is_empty() && self.board.is_shutout(color, self.game_type) {
            buf.push(Action::Pass);
        }
    }

    pub fn make(&mut self, action: &Action) -> Reversal {
        self.make_with_pinned_update(action, true)
    }

    pub(crate) fn make_leaf(&mut self, action: &Action) -> LeafReversal {
        let prev_turn_color = self.turn_color;
        let engine = match action {
            Action::Move(piece, from, to) => Some(self.board.make_leaf(*piece, Some(*from), *to)),
            Action::Place(piece, to) => Some(self.board.make_leaf(*piece, None, *to)),
            Action::Pass => None,
        };
        self.turn_color = self.turn_color.opposite_color();
        LeafReversal {
            engine,
            prev_turn_color,
        }
    }

    pub(crate) fn make_with_pinned_update(
        &mut self,
        action: &Action,
        update_pinned: bool,
    ) -> Reversal {
        let prev_turn = self.turn;
        let prev_turn_color = self.turn_color;
        let prev_last_moved = self.board.last_moved;
        let prev_last_move = self.board.last_move;
        let prev_stunned = self.board.stunned;
        let prev_hash = self.hash;
        let engine = match action {
            Action::Move(piece, from, to) => {
                self.hash ^= self.square_term(*from) ^ self.square_term(*to);
                let unmake =
                    self.board
                        .make_with_pinned_update(*piece, Some(*from), *to, update_pinned);
                self.hash ^= self.square_term(*from) ^ self.square_term(*to);
                Some(unmake)
            }
            Action::Place(piece, to) => {
                self.hash ^= self.square_term(*to);
                let unmake = self
                    .board
                    .make_with_pinned_update(*piece, None, *to, update_pinned);
                self.hash ^= self.square_term(*to);
                Some(unmake)
            }
            Action::Pass => {
                self.board.last_moved = None;
                self.board.last_move = (None, None);
                self.board.stunned = None;
                None
            }
        };
        self.turn += 1;
        self.turn_color = self.turn_color.opposite_color();
        self.hash ^= side_key();
        self.hash ^= turn_key(prev_turn) ^ turn_key(self.turn);
        if let Some(piece) = prev_stunned {
            self.hash ^= stunned_key(self.board.piece_to_offset(piece));
        }
        if let Some(piece) = self.board.stunned {
            self.hash ^= stunned_key(self.board.piece_to_offset(piece));
        }
        Reversal {
            engine,
            prev_turn,
            prev_turn_color,
            prev_last_moved,
            prev_last_move,
            prev_stunned,
            prev_hash,
        }
    }

    pub fn unmake(&mut self, reversal: Reversal) {
        self.turn = reversal.prev_turn;
        self.turn_color = reversal.prev_turn_color;
        self.hash = reversal.prev_hash;
        match reversal.engine {
            Some(engine) => self.board.unmake(engine),
            None => {
                self.board.last_moved = reversal.prev_last_moved;
                self.board.last_move = reversal.prev_last_move;
                self.board.stunned = reversal.prev_stunned;
            }
        }
    }

    pub(crate) fn unmake_leaf(&mut self, reversal: LeafReversal) {
        if let Some(engine) = reversal.engine {
            self.board.unmake_leaf(engine);
        }
        self.turn_color = reversal.prev_turn_color;
    }

    pub fn perft(&mut self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut nodes = 0;
        for action in self.legal_actions() {
            let reversal = self.make(&action);
            nodes += self.perft(depth - 1);
            self.unmake(reversal);
        }
        nodes
    }

    pub fn perft_cloning(&self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut nodes = 0;
        for action in self.legal_actions() {
            let mut child = self.clone();
            child.make(&action);
            nodes += child.perft_cloning(depth - 1);
        }
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(state: &mut State, action: &Action) -> Result<(), hudsoni::GameError> {
        match action {
            Action::Move(piece, _, to) | Action::Place(piece, to) => {
                state.play_turn_from_position(*piece, *to)
            }
            Action::Pass => state.play_turn_from_history("pass", ""),
        }
    }

    #[test]
    fn legal_actions_are_accepted_by_engine_during_selfplay() {
        let mut seed = 0x1234_5678_9abc_def0_u64;
        let mut next = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };

        for _ in 0..12 {
            let mut state = State::new(GameType::MLP, true);
            for _ply in 0..120 {
                if !matches!(state.board.game_result(), GameResult::Unknown) {
                    break;
                }
                let game = Game::from_state(&state);
                let actions = game.legal_actions();
                assert!(!actions.is_empty(), "no legal actions in a live position");
                for action in &actions {
                    let mut probe = state.clone();
                    apply(&mut probe, action)
                        .unwrap_or_else(|err| panic!("illegal action {action:?}: {err}"));
                }
                let action = actions[next() % actions.len()].clone();
                apply(&mut state, &action).expect("self-play action must be legal");
            }
        }
    }

    #[test]
    fn incremental_hash_stays_consistent_during_selfplay() {
        let mut seed = 0xC0FF_EE12_3456_789A_u64;
        let mut next = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };

        for _ in 0..16 {
            let state = State::new(GameType::MLP, true);
            let mut game = Game::from_state(&state);
            for _ply in 0..160 {
                if game.is_terminal() {
                    break;
                }
                assert_eq!(game.hash, game.compute_hash(), "incremental hash drifted");
                let actions = game.legal_actions();
                if actions.is_empty() {
                    break;
                }
                let action = actions[next() % actions.len()];
                let before = game.hash;
                let reversal = game.make(&action);
                assert_eq!(
                    game.hash,
                    game.compute_hash(),
                    "hash after make disagrees with recompute"
                );
                game.unmake(reversal);
                assert_eq!(game.hash, before, "unmake did not restore the hash");
                game.make(&action);
            }
        }
    }

    #[test]
    fn make_unmake_perft_matches_cloning_perft_on_openings() {
        for (game_type, max_depth) in [(GameType::Base, 4_usize), (GameType::MLP, 3_usize)] {
            let state = State::new(game_type, true);
            for depth in 1..=max_depth {
                let mut game = Game::from_state(&state);
                let before = game.clone();
                let journaled = game.perft(depth);
                assert_eq!(
                    game, before,
                    "{game_type} perft({depth}) must restore the position"
                );
                let cloned = game.perft_cloning(depth);
                assert_eq!(
                    journaled, cloned,
                    "{game_type} perft({depth}): make/unmake disagrees with cloning"
                );
                assert!(journaled > 0);
            }
        }
    }
}
