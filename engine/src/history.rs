use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{self, prelude::*, BufRead},
};

use crate::color::Color;
use crate::game_error::GameError;
use crate::game_result::GameResult;
use crate::game_type::GameType;

#[derive(Debug, Clone, Serialize, Default, Deserialize, PartialEq, Eq)]
pub struct History {
    pub moves: Vec<(String, String)>,
    pub result: GameResult,
    pub game_type: GameType,
}

impl fmt::Display for History {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut his = String::new();
        for (i, (piece, pos)) in self.moves.iter().enumerate() {
            his += &format!("{}. {piece} {pos}", i + 1);
        }
        write!(f, "{his}")
    }
}

impl History {
    pub fn new() -> Self {
        History {
            moves: Vec::new(),
            result: GameResult::Unknown,
            game_type: GameType::default(),
        }
    }

    pub fn new_from_gamestate(
        moves: Vec<(String, String)>,
        result: GameResult,
        game_type: GameType,
    ) -> Self {
        History {
            moves,
            result,
            game_type,
        }
    }

    pub fn new_from_str(moves: &str) -> Result<Self, GameError> {
        let mut history = History::new();
        if moves.is_empty() {
            return Ok(history);
        }
        for mov in moves.split_terminator(';') {
            let split = mov.split_whitespace().collect::<Vec<&str>>();

            let maybe_piece = split.first().ok_or(GameError::ParsingError {
                found: "NA".to_string(),
                typ: "Piece".to_string(),
            })?;

            // TODO: make sure both of them are valid Piece and Position strings

            if let Some(position) = split.get(1) {
                history
                    .moves
                    .push((maybe_piece.to_string(), position.to_string()));
            } else {
                match *maybe_piece {
                    "pass" => {
                        history.moves.push(("pass".to_string(), "".to_string()));
                    }
                    _ if history.moves.is_empty() => {
                        history
                            .moves
                            .push((maybe_piece.to_string(), ".".to_string()));
                    }
                    any => {
                        return Err(GameError::ParsingError {
                            found: any.to_owned(),
                            typ: format!("as position at turn {}", history.moves.len()),
                        })
                    }
                }
            }
        }
        Ok(history)
    }

    pub fn record_move<S1, S2>(&mut self, piece: S1, pos: S2)
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        self.moves.push((piece.into(), pos.into()));
    }

    fn parse_game_result(&mut self, str: &str) {
        match str {
            "\"1-0\"]" => self.result = GameResult::Winner(Color::White),
            "\"0-1\"]" => self.result = GameResult::Winner(Color::Black),
            "\"1/2-1/2\"]" => self.result = GameResult::Draw,
            _ => self.result = GameResult::Unknown,
        }
    }

    fn parse_game_type(&mut self, line: &str) -> Result<(), GameError> {
        lazy_static! {
            static ref GAME_TYPE: Regex = Regex::new(r#"\[GameType "(Base([+MLP]{2,4})?)"\]"#)
                .expect("This regex should compile");
        }
        if let Some(caps) = GAME_TYPE.captures(line) {
            if let Some(mtch) = caps.get(1) {
                self.game_type = mtch.as_str().parse()?;
            }
        } else {
            return Err(GameError::ParsingError {
                found: line.to_string(),
                typ: "game string".to_string(),
            });
        }
        Ok(())
    }

    fn parse_turn(&mut self, tokens: &[&str]) -> Result<(), GameError> {
        lazy_static! {
            static ref TURN: Regex = Regex::new(r"\d+").expect("This regex should compile");
        }
        if let Some(token) = tokens.first() {
            if TURN.is_match(token) {
                if let Some(piece) = tokens.get(1) {
                    if let Some(position) = tokens.get(2) {
                        self.moves.push((piece.to_string(), position.to_string()));
                    } else {
                        match *piece {
                            "pass" => {
                                self.moves.push(("pass".to_string(), "".to_string()));
                            }
                            _ if self.moves.is_empty() => {
                                self.moves.push((piece.to_string(), ".".to_string()));
                            }
                            any => {
                                return Err(GameError::ParsingError {
                                    found: any.to_owned(),
                                    typ: format!("move, in self on turn {token}"),
                                })
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn from_filepath(file_path: &str) -> Result<Self, GameError> {
        let mut history = History::new();
        lazy_static! {
            static ref HEADER: Regex = Regex::new(r"\[.*").expect("This regex should compile");
        }
        lazy_static! {
            static ref RESULT: Regex = Regex::new(r"\[Result").expect("This regex should compile");
        }
        lazy_static! {
            static ref GAME_TYPE_LINE: Regex =
                Regex::new(r"\[GameType.*").expect("This regex should compile");
        }
        match File::open(file_path) {
            Ok(file) => {
                for line in io::BufReader::new(file).lines().flatten() {
                    if line.is_empty() {
                        continue;
                    }
                    let tokens = line.split_whitespace().collect::<Vec<&str>>();
                    if RESULT.is_match(&line) {
                        if let Some(game_result) = tokens.get(1) {
                            history.parse_game_result(game_result);
                        }
                    }
                    if GAME_TYPE_LINE.is_match(&line) {
                        history.parse_game_type(&line)?;
                    }
                    if HEADER.is_match(&line) {
                        continue;
                    }
                    history.parse_turn(&tokens)?;
                }
            }
            Err(e) => {
                println!("Couldn't open file because: {e}");
            }
        }
        Ok(history)
    }

    // TODO remove once DB is online
    pub fn write_move(&self, file_name: &str, turn: usize, board_move: String) {
        let mut file = OpenOptions::new()
            .append(true)
            .open(file_name)
            .expect("game.txt cannot be written to");
        if let Err(e) = writeln!(file, "{turn}. {board_move}") {
            //TODO not sure what to do with this one
            panic!("{}", e);
        }
    }
}

