use anyhow::{anyhow, Result};
use hive_lib::{
    Color, Direction, GameError, GameResult, GameStatus, GameType, History, Piece, State,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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

#[derive(Clone, Debug)]
pub enum Outcome {
    Black,
    White,
    Draw,
}

#[derive(Clone, Debug)]
pub struct Chapter {
    hash: u64,
    game_ids: Vec<usize>,
    moves: String,
    total: usize,
    black: usize,
    white: usize,
    draws: usize,
}

impl Chapter {
    pub fn new(hash: u64, game_id: usize, moves: String, outcome: Outcome) -> Self {
        let mut chapter = Self {
            hash,
            game_ids: Vec::new(),
            moves,
            total: 0,
            black: 0,
            white: 0,
            draws: 0,
        };
        chapter.add(game_id, outcome);
        chapter
    }

    pub fn add(&mut self, game_id: usize, outcome: Outcome) {
        self.game_ids.push(game_id);
        match outcome {
            Outcome::Draw => self.draws += 1,
            Outcome::White => self.white += 1,
            Outcome::Black => self.black += 1,
        }
        self.total += 1;
    }
}

pub struct Book {
    chapters: HashMap<u64, Chapter>,
    total_games: usize,
    moves: Vec<u16>,
    games: Vec<(usize, usize)>, // start position in "moves"
    hashes: HashMap<u64, Vec<usize>>,
    // number of moves to get
    nanoid_games: HashMap<Nanoid, usize>,
    players_games: HashMap<Username, Vec<Nanoid>>,
    white_wins: Vec<Nanoid>,
    black_wins: Vec<Nanoid>,
    draws: Vec<Nanoid>,
    // TODO
    first_move: HashMap<u16, Vec<Nanoid>>,
}

pub struct Page {
    moves: Vec<u16>,
    hashes: Vec<u64>,
    white: Username,
    black: Username,
    white_win: usize,
    black_win: usize,
    draw: usize,
}

impl Page {
    pub fn new() -> Self {
        Self {
            moves: Vec::new(),
            hashes: Vec::new(),
            white: Username(String::new()),
            black: Username(String::new()),
            white_win: 0,
            black_win: 0,
            draw: 0,
        }
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Book {
    fn default() -> Self {
        Self::new()
    }
}

impl Book {
    pub fn new() -> Self {
        Self {
            chapters: HashMap::new(),
            total_games: 0,
            moves: Vec::new(),
            games: Vec::new(),
            hashes: HashMap::new(),
            nanoid_games: HashMap::new(),
            players_games: HashMap::new(),
            white_wins: Vec::new(),
            black_wins: Vec::new(),
            draws: Vec::new(),
            // these need to be implemented
            first_move: HashMap::new(),
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
                Ok(piece_u5 | destination_piece_u5 | direction_u5)
            }
        }
    }

    fn decode_move(encoded: u16) -> Result<(String, String), GameError> {
        let piece = (encoded >> 11) as u8;
        let destination_piece = ((encoded >> 6) & 0x1F) as u8;
        let direction = (encoded & 0x3F) as u8;
        if direction == 7 {
            return Ok((String::from("pass"), String::new()));
        }
        let piece = Piece::from_u5(piece);
        if destination_piece == 0 {
            return Ok((piece.to_string(), String::new()));
        }
        let destination_piece = Piece::from_u5(destination_piece);
        let direction = Direction::from_u8(direction)?;
        match direction {
            Direction::NE | Direction::E | Direction::SE => {
                Ok((piece.to_string(), format!("{destination_piece}{direction}")))
            }
            Direction::NW | Direction::W | Direction::SW => {
                Ok((piece.to_string(), format!("{direction}{destination_piece}")))
            }
        }
    }

    fn categorize_first_move(
        &mut self,
        piece: &str,
        position: &str,
        id: Nanoid,
    ) -> Result<(), GameError> {
        let encoded = Self::encode_move(piece, position)?;
        self.first_move.entry(encoded).or_default().push(id);
        Ok(())
    }

    // pub fn make_page(path: String, id: Nanoid) -> Result<Box<Page>> {
    //     let history = History::from_filepath(&path)?;
    //     if history.game_type != GameType::MLP {
    //         return Err(anyhow!("Non-PLM"));
    //     }
    //     let mut page = Page::new();
    //     let state = State::new_from_history(&history)?;
    //     match state.game_status {
    //         GameStatus::Finished(GameResult::Winner(winner)) => match winner {
    //             Color::White => page.white_win += 1,
    //             Color::Black => page.black_win += 1,
    //         },
    //         GameStatus::Finished(GameResult::Draw) => {
    //             page.draw += 1;
    //         }
    //         _ => return Err(anyhow!("Non result found")),
    //     }
    //     // if let Some((piece, position)) = history.moves.first() {
    //     //     self.categorize_first_move(piece, position, id.clone())?;
    //     // }
    //     page.hashes = state.hashes.clone();
    //     for (i, (piece, position)) in history.moves.iter().enumerate() {
    //         let encoded = Self::encode_move(piece, position)?;
    //         let (piece_decoded, position_decoded) = Self::decode_move(encoded)?;
    //         //assert_eq!(piece, &piece_decoded);
    //         //assert_eq!(position, &position_decoded);
    //         page.moves.push(encoded);
    //     }
    //     //let end = self.moves.len() - 1;
    //     //self.games.push((start, end));
    //     //self.nanoid_games.insert(id.clone(), game_id);
    //     page.black = Username(history.black);
    //     page.white = Username(history.white);
    //     Ok(Box::new(page))
    // }

    pub fn add_game_from_file(&mut self, path: String, id: Nanoid) -> Result<(), GameError> {
        let history = History::from_filepath(&path)?;
        if history.game_type != GameType::MLP {
            return Ok(());
        }
        let state = State::new_from_history(&history)?;
        let game_id = self.games.len();
        let start = self.moves.len();
        let outcome = match state.game_status {
            GameStatus::Finished(GameResult::Winner(winner)) => match winner {
                Color::White => {
                    self.white_wins.push(id.clone());
                    Outcome::White
                }
                Color::Black => {
                    self.black_wins.push(id.clone());
                    Outcome::Black
                }
            },
            GameStatus::Finished(GameResult::Draw) => {
                self.draws.push(id.clone());
                Outcome::Draw
            }
            _ => return Ok(()),
        };
        self.total_games += 1;
        if let Some((piece, position)) = history.moves.first() {
            self.categorize_first_move(piece, position, id.clone())?;
        }
        for (i, (piece, position)) in history.moves.iter().enumerate() {
            let encoded = Self::encode_move(piece, position)?;
            // let (piece_decoded, position_decoded) = Self::decode_move(encoded)?;
            // assert_eq!(piece, &piece_decoded);
            // assert_eq!(position, &position_decoded);
            let hash = state.hashes.get(i).expect("Hash to be present");
            self.hashes.entry(*hash).or_default().push(self.moves.len());
            self.moves.push(encoded);
            if i == 7 {
                let mut sub = String::new();
                for encoded in &self.moves[self.moves.len()-8..] {
                    let (piece, position) = Self::decode_move(*encoded)?;
                    sub.push_str(&format!("{} {};", piece, position));
                }
                self.chapters
                    .entry(*hash)
                    .or_insert(Chapter::new(*hash, game_id, sub, outcome.clone()))
                    .add(game_id, outcome.clone());
            }
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
        Ok(())
    }

    pub fn add_all_games(&mut self) -> Result<()> {
        let path = std::path::Path::new("./all/");
        //let entries = fs::read_dir(path)?;
        let entries: Result<Vec<_>, _> = fs::read_dir(path)?.collect();
        let entries = entries?;
        entries.iter().enumerate().for_each(|(i, entry)| {
            let entry = entry.path().display().to_string();
            //let _foo =  Self::make_page(entry, Nanoid(i));
            let _ = self.add_game_from_file(entry, Nanoid(i));
        });
        Ok(())
    }
}

fn main() {
    let mut book = Book::new();
    book.add_all_games();
    let winrate = book.white_wins.len() as f32 / book.total_games as f32;
    println!(
        "Out of {} games white won {}%",
        book.total_games,
        winrate * 100_f32
    );
    println!("White: {}", book.white_wins.len());
    println!("Black: {}", book.black_wins.len());
    println!("Draws: {}", book.draws.len());
    println!(
        "Total positions: {} unique positions: {}",
        book.moves.len(),
        book.hashes.keys().len()
    );
    for (piece, games) in book.first_move.iter() {
        let (mut win, mut loss, mut draw) = (0, 0, 0);
        for game in games.iter() {
            if book.white_wins.contains(game) {
                win += 1;
            }
            if book.black_wins.contains(game) {
                loss += 1;
            }
            if book.draws.contains(game) {
                draw += 1;
            }
        }
        let winrate = win as f32 / games.len() as f32;
        println!(
            "{:?} was played {} times, win rate: {} games: {}:{}:{}",
            Book::decode_move(*piece),
            games.len(),
            winrate,
            win,
            loss,
            draw
        );
    }
    let mut occ: Vec<usize> = book.chapters.values().map(|c| c.total).collect();
    occ.sort();
    occ.reverse();
    println!("occ: {:?}", occ);
    println!("{:?}", book.chapters);
    let cutoff = occ.get(10).expect("we have more than 10").clone();
    println!("#chapters: {}", book.chapters.len());
    for c in book.chapters.values() {
        if c.total > cutoff {
            println!(
                "#: {}, {}, white: {}%",
                c.total,
                c.moves,
                c.white as f32 / c.total as f32
            );
        }
    }
}
