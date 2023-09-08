use crate::bug::Bug;
use crate::color::Color;
use crate::game_error::GameError;
use crate::game_result::GameResult;
use crate::game_status::GameStatus;
use crate::history::History;
use crate::piece::Piece;
use crate::player::Player;
use crate::position::Position;
use crate::{board::Board, game_type::GameType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub game_id: u64,
    pub board: Board,
    pub history: History,
    pub turn: usize,
    pub turn_color: Color,
    pub players: (Player, Player),
    pub game_status: GameStatus,
    pub game_type: GameType,
    pub tournament: bool,
}

impl State {
    pub fn new(game_type: GameType, tournament: bool) -> State {
        State {
            game_id: 1,
            board: Board::new(),
            history: History::new(),
            turn: 0,
            turn_color: Color::White,
            players: (Player::new(Color::White), Player::new(Color::Black)),
            game_status: GameStatus::NotStarted,
            game_type,
            tournament,
        }
    }

    pub fn get_board(&self) -> Board {
        self.board.clone()
    }

    pub fn new_from_history(history: &History) -> Result<Self, GameError> {
        let mut tournament = true;
        // Did white open with a Queen?
        if let Some((piece_str, _)) = history.moves.get(0) {
            let piece: Piece = piece_str.parse()?;
            if piece.bug() == Bug::Queen {
                tournament = false;
            }
        }
        // Did black open with a Queen?
        if let Some((piece_str, _)) = history.moves.get(1) {
            let piece: Piece = piece_str.parse()?;
            if piece.bug() == Bug::Queen {
                tournament = false;
            }
        }
        let mut state = State::new(history.game_type, tournament);
        for (piece, pos) in history.moves.iter() {
            state.play_turn_from_notation(piece, pos)?;
        }
        Ok(state)
    }

    pub fn queen_allowed(&self) -> bool {
        self.turn > 1 || !self.tournament
    }

    pub fn play_turn_from_notation(
        &mut self,
        piece: &str,
        position: &str,
    ) -> Result<(), GameError> {
        match piece {
            "pass" => {
                if self.board.moves(self.turn_color).is_empty() {
                    self.pass();
                } else {
                    println!("Moves are: {:?}", self.board.moves(self.turn_color));
                    return Err(GameError::InvalidMove {
                        piece: "NA".to_string(),
                        from: "NA".to_string(),
                        to: "NA".to_string(),
                        turn: self.turn,
                        reason: "Trying to pass when there are available moves.".to_string(),
                    });
                }
            }
            _ => {
                let piece = piece.parse()?;
                let target_position = Position::from_string(position, &self.board)?;
                self.play_turn(piece, target_position)?;
            }
        }
        Ok(())
    }

    fn update_history(&mut self, piece: Piece, target_position: Position) {
        // if it's the first played piece on the board yet use "."
        if self.board.positions.into_iter().flatten().count() == 1 {
            self.history.record_move(piece.to_string(), ".".to_string());
            return;
        }
        if let Some((neighbor_piece, neighbor_pos)) = self.board.get_neighbor(target_position) {
            let dir = neighbor_pos.direction(target_position);
            let pos = dir.to_history_string(neighbor_piece.to_string());
            self.history.record_move(piece.to_string(), pos);
            return;
        }
        unreachable!()
    }

    fn pass(&mut self) {
        self.history
            .record_move(self.turn_color.to_string(), "pass");
        self.turn_color = Color::from(self.turn_color.opposite());
        self.turn += 1;
        self.board.last_moved = None;
    }

    fn next_turn(&mut self) {
        if self.turn == 1 {
            self.game_status = GameStatus::InProgress;
        }
        match self.board.game_result() {
            GameResult::Winner(color) => {
                self.game_status = GameStatus::Finished(GameResult::Winner(color));
                self.history.record_move(color.to_string(), "won");
                return;
            }
            GameResult::Draw => {
                self.game_status = GameStatus::Finished(GameResult::Draw);
                self.history.record_move("It's a draw", "");
                return;
            }
            GameResult::Unknown => {}
        }
        self.turn_color = Color::from(self.turn_color.opposite());
        self.turn += 1;
    }

    fn turn_move(&mut self, piece: Piece, target_position: Position) -> Result<(), GameError> {
        let mut err = GameError::InvalidMove {
            piece: piece.to_string(),
            from: "NA".to_string(),
            to: target_position.to_string(),
            turn: self.turn,
            reason: "NA".to_string(),
        };
        let current_position = self.board.position_of_piece(piece).ok_or({
            err.update_reason("This piece is not on the board.");
            err.clone()
        })?;
        err.update_from(current_position.to_string());
        if self.board.is_pinned(piece) {
            err.update_reason("Piece is pinned.");
            return Err(err);
        }
        // remove the piece from its current location
        if !self
            .board
            .is_valid_move(self.turn_color, piece, current_position, target_position)
        {
            println!("Board state is: {}", self.board);
            err.update_reason("This move isn't valid.");
            return Err(err);
        }
        self.board
            .move_piece(piece, current_position, target_position, self.turn)?;
        Ok(())
    }

    pub fn turn_spawn(&mut self, piece: Piece, target_position: Position) -> Result<(), GameError> {
        let mut err = GameError::InvalidMove {
            piece: piece.to_string(),
            from: "Reserve".to_string(),
            to: target_position.to_string(),
            turn: self.turn,
            reason: "NA".to_string(),
        };
        if !piece.is_color(self.turn_color) {
            err.update_reason(format!(
                "It is {}'s turn, but {} tried to spawn a piece.",
                self.turn_color,
                piece.color()
            ));
            return Err(err);
        }
        if self.turn < 2 && piece.bug() == Bug::Queen && self.tournament {
            err.update_reason("Can't spawn Queen. Game uses tournament rules");
            return Err(err);
        }
        if piece.bug() != Bug::Queen && self.board.queen_required(self.turn, piece.color()) {
            err.update_reason("Can't spawn another piece. Queen is required.");
            return Err(err);
        }
        if self.board.spawnable(piece.color(), target_position) {
            self.board.insert(target_position, piece);
        } else {
            err.update_reason(format!("{} is not allowed to spawn here.", self.turn_color));
            return Err(err);
        }
        Ok(())
    }

    pub fn play_turn(&mut self, piece: Piece, target_position: Position) -> Result<(), GameError> {
        // TODO check for GameStatus::Finished
        if self.board.piece_already_played(piece) {
            self.turn_move(piece, target_position)?
        } else {
            self.turn_spawn(piece, target_position)?
        }
        self.update_history(piece, target_position);
        debug_assert!(self.board.check());
        self.next_turn();
        Ok(())
    }

    pub fn check_board(&self) -> bool {
        // This function can be used to perform checks on the engine and for debugging engine
        // issues on every turn
        //true
        // for this remove the return true and then implement your check in the loop
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                let hex = self.board.board.get(position);
                let neighbor_count = *self.board.neighbor_count.get(position);
                let counted = self.board.positions_taken_around(position).count();
                if counted != neighbor_count as usize {
                    println!("Calculated: {counted} hashed: {neighbor_count}");
                    println!("turn: {}", self.turn);
                    println!("pos: {position}");
                    println!("hex: {hex:?}");
                    println!("{}", self.board);
                    return false;
                }
            }
        }
        true
    }
}
