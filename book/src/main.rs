use hive_lib::{
    Color, Direction, GameError, GameResult, GameStatus, GameType, History, Piece, State,
};
use std::collections::HashMap;
use std::fs;

#[derive(PartialEq, Eq, Hash, Clone)]
struct Nanoid(usize);

#[derive(PartialEq, Eq, Hash)]
struct Username(String);
// 8 distinct bugs
// 2 colors
// 14 bugs each
// 28 bugs total
// 28 bugs <= 32, 32 = 5 bits
// A move is 2 bugs 1 position
// A move is 5 bits + 5 bits + Position
// Position: NE, NW, W, SW, SE, E NE = 6
// encode: black/white/draw/pass = 4
// A move can therefore be encoded in 16bits
// A game is on average 100 moves
// 1600 bits or 200bytes
// 200 bytes is 200 ASCII characters
// So all human vs human PLM games played on BS can be encoded in 2MB

pub struct Book {
    total_games: usize,
    moves: Vec<u16>,
    games: Vec<(usize, usize)>, // start position in "moves"
    // number of moves to get
    nanoid_games: HashMap<Nanoid, usize>,
    players_games: HashMap<Username, Vec<Nanoid>>,
    white_won: Vec<Nanoid>,
    black_won: Vec<Nanoid>,
    draws: Vec<Nanoid>,
    // TODO
    // wp_games: Vec<Nanoid>,
    // wl_games: Vec<Nanoid>,
    // buffer_games: Vec<Nanoid>,
}

impl Default for Book {
    fn default() -> Self {
        Self::new()
    }
}

impl Book {
    pub fn new() -> Self {
        Self {
            total_games: 0,
            moves: Vec::new(),
            games: Vec::new(),
            nanoid_games: HashMap::new(),
            players_games: HashMap::new(),
            white_won: Vec::new(),
            black_won: Vec::new(),
            draws: Vec::new(),
            // these need to be implemented
            // wp_games: Vec::new(),
            // wl_games: Vec::new(),
            // buffer_games: Vec::new(),
        }
    }

    fn encode_move(piece: &str, position: &str) -> Result<u16, GameError> {
        match piece {
            "pass" => Ok(7),
            _ => {
                // 01234, 56789, a..f
                // piece, piece, pass, direction
                let piece: Piece = piece.parse()?;
                let piece_u5 = ((piece.to_u5() as u32) << 11) as u16;
                let mut direction_u5 = 0;
                if let Some(direction) = Direction::from_string(position) {
                    direction_u5 = direction.to_u8() as u16;
                }
                let mut position = position.to_string();
                position.retain(|c| c != '\\' && c != '/' && c != '-');
                let mut destination_piece_u5 = 0;
                if !position.is_empty() {
                    let destination_piece: Piece = position.parse()?;
                    destination_piece_u5 = ((destination_piece.to_u5() as u32) << 6) as u16;
                }
                Ok(piece_u5 & destination_piece_u5 & direction_u5)
            }
        }
    }

    pub fn add_game_from_file(&mut self, path: String, id: Nanoid) -> Result<(), GameError> {
        let history = History::from_filepath(&path)?;
        let state = State::new_from_history(&history)?;
        let game_id = self.games.len();
        let start = self.moves.len();
        for (piece, position) in &history.moves {
            self.moves.push(Self::encode_move(piece, position)?);
        }
        let end = self.moves.len() - 1;
        self.games.push((start, end));
        self.nanoid_games.insert(id.clone(), game_id);
        self.players_games
            .entry(Username(history.black))
            .or_default()
            .push(id.to_owned());
        self.players_games
            .entry(Username(history.white))
            .or_default()
            .push(id.to_owned());
        match state.game_status {
            GameStatus::Finished(GameResult::Winner(winner)) => match winner {
                Color::White => self.white_won.push(id),
                Color::Black => self.black_won.push(id),
            },
            GameStatus::Finished(GameResult::Draw) => {
                self.draws.push(id);
            }
            _ => {}
        }
        self.total_games += 1;
        Ok(())
    }

    pub fn add_all_games(&mut self) -> Result<(), GameError> {
        for (i, entry) in fs::read_dir("./2023/")
            .expect("Should be valid directory")
            .enumerate()
        {
            let entry = entry.expect("PGN").path().display().to_string();
            //println!("Adding: {entry}");

            match self.add_game_from_file(entry, Nanoid(i)) {
                Ok(_) => {}
                Err(err) => {}, //println!("Error adding file: {}", err),
            }
        }
        Ok(())
    }
}

fn main() {
    let mut book = Book::new();
    book.add_all_games();
    println!("White: {}", book.white_won.len());
    println!("Black: {}", book.black_won.len());
    println!("Draws: {}", book.draws.len());
    println!("Moves: {}", book.moves.len());
}
