use crate::{
    color::Color, game_error::GameError, game_result::GameResult, game_status::GameStatus,
    game_type::GameType, state::State,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

lazy_static! {
    static ref HEADER: Regex = Regex::new(r"\[.*").expect("This regex should compile");
    static ref RESULT: Regex = Regex::new(r"\[Result").expect("This regex should compile");
    static ref GAME_TYPE_LINE: Regex =
        Regex::new(r"\[GameType.*").expect("This regex should compile");
    static ref UHP_TURN: Regex =
        Regex::new(r"^(White|Black)\[\d+\]$").expect("This regex should compile");
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, PartialEq, Eq)]
pub struct History {
    pub moves: Vec<(String, String)>,
    pub hashes: Vec<u64>,
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
            hashes: Vec::new(),
            result: GameResult::Unknown,
            game_type: GameType::default(),
        }
    }

    pub fn new_from_gamestate(
        moves: Vec<(String, String)>,
        hashes: &[u64],
        result: GameResult,
        game_type: GameType,
    ) -> Self {
        History {
            hashes: hashes.to_owned(),
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
            history.push_move_tokens(&split)?;
        }
        Ok(history)
    }

    pub fn record_hash(&mut self, hash: u64) {
        self.hashes.push(hash)
    }

    pub fn record_move<S1, S2>(&mut self, piece: S1, pos: S2)
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        self.moves.push((piece.into(), pos.into()));
    }

    pub fn move_is_pass(&self, turn: usize) -> bool {
        if let Some(mov) = self.moves.get(turn) {
            return mov.0 == "pass";
        }
        false
    }

    pub fn last_move_is_pass(&self) -> bool {
        if let Some(mov) = self.moves.last() {
            return mov.0 == "pass";
        }
        false
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
                                self.moves.push((piece.to_string(), "".to_string()));
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

    pub fn from_filepath(file_path: PathBuf) -> Result<Self, GameError> {
        let mut history = History::new();
        match File::open(file_path) {
            Ok(file) => {
                for line in io::BufReader::new(file).lines().map_while(Result::ok) {
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

    pub fn from_pgn_str(string: String) -> Result<Self, GameError> {
        let mut history = History::new();
        for line in string.lines() {
            if line.is_empty() {
                continue;
            }
            let tokens = line.split_whitespace().collect::<Vec<&str>>();
            if RESULT.is_match(line) {
                if let Some(game_result) = tokens.get(1) {
                    history.parse_game_result(game_result);
                }
            }
            if GAME_TYPE_LINE.is_match(line) {
                history.parse_game_type(line)?;
            }
            if HEADER.is_match(line) {
                continue;
            }
            history.parse_turn(&tokens)?;
        }
        Ok(history)
    }

    pub fn from_uhp_str(string: String) -> Result<Self, GameError> {
        let mut history = History::new();
        let trimmed = string.trim();
        if trimmed.is_empty() {
            return Ok(history);
        }

        let parts: Vec<&str> = trimmed
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if parts.is_empty() {
            return Ok(history);
        }

        let mut index = 0;

        if let Some(token) = parts.get(index) {
            if token.starts_with("Base") {
                history.game_type = token.parse()?;
                index += 1;
            }
        }

        if let Some(token) = parts.get(index) {
            if let Some(result) = Self::parse_uhp_game_status(token) {
                history.result = result;
                index += 1;
            }
        }

        if let Some(token) = parts.get(index) {
            if UHP_TURN.is_match(token) {
                index += 1;
            } else if Self::looks_like_metadata(token) {
                return Err(GameError::ParsingError {
                    found: (*token).to_string(),
                    typ: "UHP metadata string".to_string(),
                });
            }
        }

        while let Some(token) = parts.get(index) {
            let split = token.split_whitespace().collect::<Vec<&str>>();
            if split.is_empty() {
                index += 1;
                continue;
            }
            if let Err(err) = history.push_move_tokens(&split) {
                let turn = history.moves.len();
                return Err(GameError::PartialHistory {
                    history,
                    turn,
                    reason: err.to_string(),
                });
            }
            if let Some(piece) = split.first() {
                history.upgrade_game_type_for_piece(piece);
            }
            index += 1;
        }

        let mut playback_history = history.clone();
        playback_history.result = GameResult::Unknown;
        match State::new_from_history(&playback_history) {
            Ok(state) => {
                history.result = match state.game_status {
                    GameStatus::Finished(result) => result,
                    _ => GameResult::Unknown,
                };
            }
            Err(err) => {
                //TODO: fix engine/src/game_error.rs so we can just use the turn from there instead of replaying the game in case of an error
                let (turn, reason) = Self::failing_turn(&history)
                    .map(|(turn, err)| (turn, err.to_string()))
                    .unwrap_or_else(|| (history.moves.len(), err.to_string()));
                history.moves.truncate(turn);
                return Err(GameError::PartialHistory {
                    history,
                    turn,
                    reason,
                });
            }
        }

        Ok(history)
    }

    fn push_move_tokens(&mut self, split: &[&str]) -> Result<(), GameError> {
        let maybe_piece = split.first().ok_or(GameError::ParsingError {
            found: split.join(" "),
            typ: "Piece".to_string(),
        })?;

        if let Some(position) = split.get(1) {
            self.moves
                .push((maybe_piece.to_string(), position.to_string()));
        } else {
            match *maybe_piece {
                "pass" => {
                    self.moves.push(("pass".to_string(), "".to_string()));
                }
                _ if self.moves.is_empty() => {
                    self.moves.push((maybe_piece.to_string(), "".to_string()));
                }
                any => {
                    return Err(GameError::ParsingError {
                        found: any.to_owned(),
                        typ: format!("as position at turn {}", self.moves.len()),
                    })
                }
            }
        }
        Ok(())
    }

    fn upgrade_game_type_for_piece(&mut self, piece: &str) {
        if piece == "pass" {
            return;
        }
        let mut chars = piece.chars();
        match (chars.next(), chars.next()) {
            (Some('w') | Some('b'), Some('M')) => {
                self.game_type = self.game_type.add_m();
            }
            (Some('w') | Some('b'), Some('L')) => {
                self.game_type = self.game_type.add_l();
            }
            (Some('w') | Some('b'), Some('P')) => {
                self.game_type = self.game_type.add_p();
            }
            _ => {}
        }
    }

    fn parse_uhp_game_status(token: &str) -> Option<GameResult> {
        match token {
            "NotStarted" | "InProgress" => Some(GameResult::Unknown),
            "Draw" => Some(GameResult::Draw),
            "WhiteWins" => Some(GameResult::Winner(Color::White)),
            "BlackWins" => Some(GameResult::Winner(Color::Black)),
            _ => None,
        }
    }

    fn looks_like_metadata(token: &str) -> bool {
        token
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
    }

    fn failing_turn(history: &History) -> Option<(usize, GameError)> {
        let mut state = State::new(history.game_type, false);
        for (turn, (piece, position)) in history.moves.iter().enumerate() {
            if let Err(err) = state.play_turn_from_history(piece, position) {
                return Some((turn, err));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_METADATA_GAME: &str = r#"Base+MLP;InProgress;White[29];wA1;bA1 /wA1;wQ \wA1;bA2 /bA1;wA2 wQ/;bL bA1\;wA3 wA2\;bQ -bA1;wA2 /bA2;bL -wA2;wA3 -bQ;bA3 -bL;wM wA1-;bA3 -wA3;wM bA2\;bM \bA3;wM -bM;bP bM/;wG1 /wM;bP \wA3;wG2 wG1\;bA3 bL\;wS1 wA1/;bS1 bP/;wS1 bA1\;bS1 -wQ;wL wA1-;bS1 \wL;wQ \bS1;bG1 bP/;wG3 -wM;bA3 \wQ;wL -wA1;bS2 bG1/;wB1 wA2\;bS2 /bA3;wL bS1\;bG2 -bA3;wL bS2\;bG2 wQ/;wB1 wA2;bG3 -bS2;wL -wA1;wA3 -bG1;wB1 /bQ;bP -wA3;wB1 bQ;bB1 bS1-;wB2 -wB1;bM wA2\;wB2 /wB1;bL -wB2;wS2 wM\;bB2 bB1/;wS2 -wB1;bM \wA3"#;
    const IMPLICIT_METADATA_GAME: &str = r#"wA1;bG1 -wA1;wQ wA1/;bA1 -bG1;wA2 \wQ;bQ bA1\;wA2 \bA1;bA2 bQ\;wA3 wA1\;bA2 /wA2;wA3 /bA2;bA3 /bQ;wA2 bA3\;bG2 \bA2;wQ \wA1;bG2 bA2\;wA1 \bA2;bA1 /wA3;wS1 wA2\;bB1 -bG1;wS1 bG1\;bG2 -wQ;wQ bG2/;bG1 -wQ;wQ bG1/;bS1 bG2-;wG1 \wA1;bA1 wA2-;wG1 bA2\;bS1 wQ/;wA3 bA1-;bS2 bS1-;wG2 wS1/;bS2 -bS1;wG2 bQ\;bA3 -wA2;wB1 wS1/;bB2 bS1-;wB1 wS1;bB2 bS1;wB1 bQ;bG3 bG2-;wB1 wG1\;bA3 bG3-;wA2 bA3\;bB2 bS2;wG3 /wB1;bS1 \bA3;"#;
    const BASE_DECLARED_GAME: &str = r#"Base;InProgress;Black[18];wL;bL wL\;wQ -wL;bA1 bL\;wA1 wL/;bA2 bA1/;wA2 \wA1;bQ bA2\;wA2 /bA1;bA2 -wQ;wA3 /wA2;bP /bA2;wA3 bQ/;bP bA2\;wS1 wA3-;bG1 /bA2;wA2 wA3\;bA3 -bA2;wA1 -wA3;bQ /wA2;wG1 \wS1;bA3 \wG1;wS1 bQ\;bG2 bG1\;wM wG1\;bG3 -bA3;wM \bA2;bG2 \wL;wM /bA1;bS1 bP\;wG2 /wS1;bG1 -bG2;wM /bS1;bS2 /bA2;wG3 wG1\"#;
    const RESULT_MISMATCH_GAME: &str = "Base;WhiteWins;White[2];wS1;bS1 wS1-;";
    const PARTIAL_GAME: &str = "Base;InProgress;White[2];wS1;bS1 wS1-;wQ bad_input;bQ -bS1;wQ wS1/";
    const QUEEN_FIRST_BAD: &str = r#"wQ;bG1 -wQ;wA1 wQ/;bA1 -bG1;bad_input bad_input"#;

    #[test]
    fn parses_full_metadata_game_string() {
        let history = History::from_uhp_str(FULL_METADATA_GAME.to_string()).expect("valid UHP");
        assert_eq!(history.moves.len(), 56);
        assert_eq!(history.game_type, GameType::MLP);
        assert_eq!(history.result, GameResult::Unknown);
    }

    #[test]
    fn parses_moves_when_metadata_missing() {
        let history =
            History::from_uhp_str(IMPLICIT_METADATA_GAME.to_string()).expect("implicit UHP");
        assert_eq!(history.moves.len(), 48);
        assert_eq!(history.game_type, GameType::Base);
        assert_eq!(history.result, GameResult::Unknown);
        assert_eq!(history.moves.first().map(|m| m.0.as_str()), Some("wA1"));
    }

    #[test]
    fn upgrades_game_type_based_on_moves() {
        let history =
            History::from_uhp_str(BASE_DECLARED_GAME.to_string()).expect("upgrade game type");
        assert_eq!(history.moves.len(), 35);
        assert_eq!(history.game_type, GameType::MLP);
    }

    #[test]
    fn prefers_computed_result_over_declared_state() {
        let history =
            History::from_uhp_str(RESULT_MISMATCH_GAME.to_string()).expect("mismatch resilience");
        assert_eq!(history.moves.len(), 2);
        assert_eq!(history.result, GameResult::Unknown);
    }

    #[test]
    fn returns_partial_history_error() {
        match History::from_uhp_str(PARTIAL_GAME.to_string()) {
            Err(GameError::PartialHistory {
                history,
                turn,
                reason,
            }) => {
                assert_eq!(turn, 2);
                assert_eq!(history.moves.len(), 2);
                assert_eq!(history.moves[0].0, "wS1");
                assert_eq!(history.moves[1].0, "bS1");
                assert!(reason.contains("bad_input"));
            }
            other => panic!("expected partial history error, got {other:?}"),
        }
    }

    #[test]
    fn q_first_still_returns_partial_history_error() {
        match History::from_uhp_str(QUEEN_FIRST_BAD.to_string()) {
            Err(GameError::PartialHistory {
                history,
                turn,
                reason,
            }) => {
                assert_eq!(turn, 4);
                assert_eq!(history.moves.len(), 4);
                assert_eq!(history.moves[0].0, "wQ");
                assert!(reason.contains("bad_input"));
            }
            other => panic!("expected partial history error, got {other:?}"),
        }
    }
}
