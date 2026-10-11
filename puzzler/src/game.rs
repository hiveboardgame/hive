use hive_lib::{Board, Bug, Color, GameResult, GameType, Piece, Position, State, Unmake};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Move {
        piece: Piece,
        from: Position,
        to: Position,
    },
    Place {
        piece: Piece,
        to: Position,
    },
    Pass,
}

impl Action {
    pub fn piece(&self) -> Option<Piece> {
        match self {
            Action::Move { piece, .. } | Action::Place { piece, .. } => Some(*piece),
            Action::Pass => None,
        }
    }

    pub fn to(&self) -> Option<Position> {
        match self {
            Action::Move { to, .. } | Action::Place { to, .. } => Some(*to),
            Action::Pass => None,
        }
    }

    pub fn from(&self) -> Option<Position> {
        match self {
            Action::Move { from, .. } => Some(*from),
            _ => None,
        }
    }
}

pub struct Undo {
    board: Option<Unmake>,
    last_moved: Option<(Piece, Position)>,
    last_move: (Option<Position>, Option<Position>),
    stunned: Option<Piece>,
    key: u64,
}

/// A search view of a position: the engine's board plus the little turn state `State` keeps,
/// played with make/unmake. Repetition is deliberately absent - a forced win is proven without it.
#[derive(Clone)]
pub struct Game {
    pub board: Board,
    pub game_type: GameType,
    pub to_move: Color,
    pub turn: usize,
    pub tournament: bool,
    key: u64,
}

const SIDE_KEY: u64 = 0x9e37_79b9_7f4a_7c15;

fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn square_key(piece: Piece, at: Position, level: usize) -> u64 {
    let packed = (u64::from(u8::from(piece)) << 40)
        | ((at.q as u16 as u64) << 24)
        | ((at.r as u16 as u64) << 8)
        | level as u64;
    mix(packed ^ 0x5bd1_e995)
}

fn last_moved_key(last_moved: Option<(Piece, Position)>) -> u64 {
    match last_moved {
        Some((piece, at)) => mix(square_key(piece, at, 99) ^ 0x2545_f491),
        None => 0,
    }
}

impl Game {
    pub fn from_state(state: &State) -> Self {
        let mut game = Self {
            board: state.board.clone(),
            game_type: state.game_type,
            to_move: state.turn_color,
            turn: state.turn,
            tournament: state.tournament,
            key: 0,
        };
        game.key = game.compute_key();
        game
    }

    fn compute_key(&self) -> u64 {
        let mut key = if self.to_move == Color::Black {
            SIDE_KEY
        } else {
            0
        };
        for at in self.board.all_taken_positions() {
            for (level, piece) in self.board.board.get(at).pieces.iter().enumerate() {
                if level < self.board.level(at) {
                    key ^= square_key(*piece, at, level);
                }
            }
        }
        key ^ last_moved_key(self.board.last_moved)
    }

    pub fn key(&self) -> u64 {
        self.key
    }

    pub fn result(&self) -> GameResult {
        self.board.game_result()
    }

    pub fn is_over(&self) -> bool {
        self.result() != GameResult::Unknown
    }

    fn queen_allowed(&self) -> bool {
        self.turn > 1 || !self.tournament
    }

    /// Every legal action for the side to move; a forced pass when nothing else is legal.
    pub fn actions(&self, buf: &mut Vec<Action>) {
        buf.clear();
        if self.is_over() {
            return;
        }
        let color = self.to_move;
        self.board.for_each_move(color, |piece, from, to| {
            buf.push(Action::Move { piece, from, to })
        });
        self.board
            .for_each_placement(color, self.game_type, self.queen_allowed(), |piece, to| {
                buf.push(Action::Place { piece, to })
            });
        if buf.is_empty() {
            buf.push(Action::Pass);
        }
    }

    /// Every action that surrounds the enemy Queen right now, without generating all moves: only
    /// her last open cell can be filled, so only moves into it are tried.
    pub fn immediate_wins(&self, out: &mut Vec<Action>) {
        out.clear();
        if self.is_over() {
            return;
        }
        let attacker = self.to_move;
        let defender = attacker.opposite_color();
        let Some(queen_at) = self.queen_position(defender) else {
            return;
        };
        let wins = |action: &Action| self.result_after(action) == GameResult::Winner(attacker);
        // A pinned Queen cannot be thrown this move.
        let queen_free = !self.board.is_pinned(Self::queen(defender));
        if queen_free && self.queen_throw_risk_after(defender, &Action::Pass) {
            let mut all = Vec::new();
            self.actions(&mut all);
            out.extend(all.into_iter().filter(|action| wins(action)));
            return;
        }
        let mut open = queen_at
            .positions_around()
            .filter(|around| !self.board.occupied(*around));
        let (Some(cell), None) = (open.next(), open.next()) else {
            return;
        };
        for from in self.board.all_taken_positions() {
            let Some(piece) = self.board.top_piece(from) else {
                continue;
            };
            if self.board.is_valid_move(attacker, piece, from, cell) {
                let action = Action::Move {
                    piece,
                    from,
                    to: cell,
                };
                if wins(&action) {
                    out.push(action);
                }
            }
        }
        self.board.for_each_placement(
            attacker,
            self.game_type,
            self.queen_allowed(),
            |piece, to| {
                let action = Action::Place { piece, to };
                if to == cell && wins(&action) {
                    out.push(action);
                }
            },
        );
    }

    pub fn result_after(&self, action: &Action) -> GameResult {
        match action {
            Action::Move { piece, from, to } => self.board.result_after(*piece, Some(*from), *to),
            Action::Place { piece, to } => self.board.result_after(*piece, None, *to),
            Action::Pass => GameResult::Unknown,
        }
    }

    /// Neighbours of `queen`'s cell once `action` is played, or None while she is unplayed.
    pub fn queen_neighbors_after(&self, queen: Piece, action: &Action) -> Option<usize> {
        let mut at = self.board.position_of_piece(queen)?;
        let (vacated, filled) = match action {
            Action::Move { piece, from, to } => {
                if *piece == queen {
                    at = *to;
                }
                (
                    (self.board.level(*from) == 1).then_some(*from),
                    (!self.board.occupied(*to)).then_some(*to),
                )
            }
            Action::Place { to, .. } => (None, Some(*to)),
            Action::Pass => (None, None),
        };
        Some(
            at.positions_around()
                .filter(|around| {
                    (self.board.occupied(*around) && Some(*around) != vacated)
                        || Some(*around) == filled
                })
                .count(),
        )
    }

    /// Whether `attacker` might be able to throw `color`'s Queen right after `action`: a Pillbug
    /// or Mosquito of theirs touching her. Conservative - it answers true whenever in doubt, because
    /// it guards prunes that assume a Queen stays where she is.
    pub fn queen_throw_risk_after(&self, color: Color, action: &Action) -> bool {
        let queen = Self::queen(color);
        let Some(mut at) = self.board.position_of_piece(queen) else {
            return false;
        };
        if action.piece() == Some(queen) {
            if let Some(to) = action.to() {
                at = to;
            }
        }
        if action.from().is_some_and(|from| from.is_neighbor(at)) {
            return true;
        }
        let thrower = |piece: Piece| {
            piece.color() != color && matches!(piece.bug(), Bug::Pillbug | Bug::Mosquito)
        };
        if action.piece().is_some_and(thrower) && action.to().is_some_and(|to| to.is_neighbor(at)) {
            return true;
        }
        at.positions_around()
            .filter_map(|around| self.board.top_piece(around))
            .any(thrower)
    }

    pub fn make(&mut self, action: &Action) -> Undo {
        let mut undo = Undo {
            board: None,
            last_moved: self.board.last_moved,
            last_move: self.board.last_move,
            stunned: self.board.stunned,
            key: self.key,
        };
        self.key ^= last_moved_key(self.board.last_moved);
        match action {
            Action::Move { piece, from, to } => {
                self.key ^= square_key(*piece, *from, self.board.level(*from) - 1);
                self.key ^= square_key(*piece, *to, self.board.level(*to));
                undo.board = Some(self.board.make(*piece, Some(*from), *to, self.to_move));
            }
            Action::Place { piece, to } => {
                self.key ^= square_key(*piece, *to, self.board.level(*to));
                undo.board = Some(self.board.make(*piece, None, *to, self.to_move));
            }
            Action::Pass => {
                self.board.last_moved = None;
                self.board.last_move = (None, None);
                self.board.stunned = None;
            }
        }
        self.key ^= last_moved_key(self.board.last_moved);
        self.key ^= SIDE_KEY;
        self.to_move = self.to_move.opposite_color();
        self.turn += 1;
        undo
    }

    pub fn unmake(&mut self, undo: Undo) {
        self.to_move = self.to_move.opposite_color();
        self.turn -= 1;
        match undo.board {
            Some(unmake) => self.board.unmake(unmake),
            None => {
                self.board.last_moved = undo.last_moved;
                self.board.last_move = undo.last_move;
                self.board.stunned = undo.stunned;
            }
        }
        self.key = undo.key;
    }

    pub fn queen(color: Color) -> Piece {
        Piece::new_from(Bug::Queen, color, 0)
    }

    pub fn queen_position(&self, color: Color) -> Option<Position> {
        self.board.position_of_piece(Self::queen(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        crate::prove::tests::fixtures()
            .into_iter()
            .find(|(state, _)| state.game_type == hive_lib::GameType::MLP)
            .expect("an MLP fixture")
            .0
    }

    #[test]
    fn make_then_unmake_restores_board_and_key() {
        let state = state();
        let mut game = Game::from_state(&state);
        let before_board = game.board.clone();
        let before_key = game.key();
        let mut actions = Vec::new();
        game.actions(&mut actions);
        assert!(!actions.is_empty());
        for action in actions {
            let undo = game.make(&action);
            assert_eq!(
                game.key(),
                game.compute_key(),
                "incremental key after {action:?}"
            );
            game.unmake(undo);
            assert_eq!(game.board, before_board, "board after undoing {action:?}");
            assert_eq!(game.key(), before_key, "key after undoing {action:?}");
        }
    }

    #[test]
    fn queen_neighbors_after_matches_making_the_move() {
        let state = state();
        let mut game = Game::from_state(&state);
        let mut actions = Vec::new();
        game.actions(&mut actions);
        for color in [Color::White, Color::Black] {
            let queen = Game::queen(color);
            for action in &actions {
                let predicted = game.queen_neighbors_after(queen, action);
                let undo = game.make(action);
                let actual = game.queen_position(color).map(|at| {
                    at.positions_around()
                        .filter(|around| game.board.occupied(*around))
                        .count()
                });
                game.unmake(undo);
                assert_eq!(predicted, actual, "{color} queen after {action:?}");
            }
        }
    }
}
