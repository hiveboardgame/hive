use std::{
    collections::HashMap,
    iter::Peekable,
    str::{Chars, FromStr},
};

use crate::{
    board::Board,
    bug::Bug,
    color::Color,
    direction::Direction,
    game_type::GameType,
    piece::Piece,
    position::{Position, Rotation},
};

use super::{
    error::HopError,
    frame::{HOP_MINUS, HOP_PLUS},
};

#[derive(Debug, Clone)]
pub struct HopPosition {
    pub board: Board,
    pub game_type: GameType,
    pub to_move: Color,
}

/// Parse a HOP string into a concrete board, game type and side to move. White always
/// opens, so with exactly one piece on the board that piece must be White and Black must
/// be to move — anything else describes an unreachable position and is rejected.
pub fn parse(hop: &str) -> Result<HopPosition, HopError> {
    let hop = hop.trim();
    if hop.is_empty() {
        return Err(HopError::Empty);
    }
    let fields: Vec<&str> = hop.split(',').collect();
    let (game_type, topology, player) = match fields.as_slice() {
        [topology, player] => (GameType::MLP, *topology, *player),
        [game, topology, player] => (parse_game_type(game)?, *topology, *player),
        other => return Err(HopError::FieldCount(other.len())),
    };
    let to_move = parse_player(player)?;
    let board = parse_topology(topology, game_type)?;
    if board.played == 1 {
        let position = board
            .all_taken_positions()
            .next()
            .expect("played == 1 implies one taken position");
        let piece_color = board
            .top_piece(position)
            .expect("position is taken")
            .color();
        if piece_color != Color::White || to_move != Color::Black {
            return Err(HopError::LoneWhitePieceRequired);
        }
    }
    Ok(HopPosition {
        board,
        game_type,
        to_move,
    })
}

/// Canonical position hash of a HOP string; `stunned` lets the caller supply
/// stun state that HOP's grammar can't express.
pub fn to_hash(hop: &str, stunned: Option<Piece>) -> Result<i64, HopError> {
    let mut parsed = parse(hop)?;
    Ok(parsed.board.position_hash(parsed.to_move, stunned) as i64)
}

fn parse_game_type(field: &str) -> Result<GameType, HopError> {
    let normalized = field.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "ultimate" {
        return Ok(GameType::MLP);
    }
    let modifiers = normalized
        .strip_prefix("base")
        .ok_or_else(|| HopError::UnsupportedGameType(field.to_string()))?;
    let (mut m, mut l, mut p) = (false, false, false);
    for ch in modifiers.chars() {
        match ch {
            '+' => {}
            'm' => m = true,
            'l' => l = true,
            'p' => p = true,
            'd' => return Err(HopError::Dragonfly),
            _ => return Err(HopError::UnsupportedGameType(field.to_string())),
        }
    }
    Ok(match (m, l, p) {
        (false, false, false) => GameType::Base,
        (true, false, false) => GameType::M,
        (false, true, false) => GameType::L,
        (false, false, true) => GameType::P,
        (true, true, false) => GameType::ML,
        (false, true, true) => GameType::LP,
        (true, false, true) => GameType::MP,
        (true, true, true) => GameType::MLP,
    })
}

/// `w`/`b` with an optional, ignored orientation suffix: at most one rotation
/// digit `0`–`5`, then an optional single mirror flag `m`/`M`; nothing else is permitted.
fn parse_player(field: &str) -> Result<Color, HopError> {
    let field = field.trim();
    let mut chars = field.chars();
    let color = match chars.next() {
        Some('w' | 'W') => Color::White,
        Some('b' | 'B') => Color::Black,
        _ => return Err(HopError::BadPlayer(field.to_string())),
    };
    let mut rotated = false;
    let mut mirrored = false;
    for ch in chars {
        match ch {
            '0'..='5' if !rotated && !mirrored => rotated = true,
            'm' | 'M' if !mirrored => mirrored = true,
            _ => return Err(HopError::BadPlayer(field.to_string())),
        }
    }
    Ok(color)
}

fn parse_topology(topology: &str, game_type: GameType) -> Result<Board, HopError> {
    if topology.is_empty() {
        return Ok(Board::new());
    }
    let mut walk = Walk::new(game_type);
    let mut chars = topology.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '+' => walk.turn(HOP_PLUS),
            '-' => walk.turn(HOP_MINUS),
            '!' => walk.mark_last_moved(),
            '(' => walk.open_scope(),
            ')' => walk.close_scope()?,
            'd' | 'D' => return Err(HopError::Dragonfly),
            c if c.is_ascii_alphabetic() => walk.place(c)?,
            c if c.is_ascii_digit() => {
                let n = read_number(c, &mut chars)?;
                walk.chain_ref(n, &mut chars)?;
            }
            other => return Err(HopError::BadChar(other)),
        }
    }
    walk.finish()
}

struct Walk {
    board: Board,
    game_type: GameType,
    counts: HashMap<(Color, Bug), usize>,
    /// One frame per open `(…)`; a chain reference `N` indexes the innermost frame, 1-based.
    scopes: Vec<Vec<(Position, Direction)>>,
    position: Position,
    heading: Direction,
    placed_any: bool,
    last_placed: Option<(Piece, Position)>,
    marked: Option<(Piece, Position)>,
}

impl Walk {
    fn new(game_type: GameType) -> Self {
        Walk {
            board: Board::new(),
            game_type,
            counts: HashMap::new(),
            scopes: vec![Vec::new()],
            position: Position::initial_spawn_position(),
            heading: Direction::E,
            placed_any: false,
            last_placed: None,
            marked: None,
        }
    }

    fn turn(&mut self, rotation: Rotation) {
        self.heading = self.heading.next_direction(rotation);
    }

    fn mark_last_moved(&mut self) {
        self.marked = self.last_placed;
    }

    fn open_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    fn close_scope(&mut self) -> Result<(), HopError> {
        if self.scopes.len() == 1 {
            return Err(HopError::UnbalancedParens);
        }
        self.scopes.pop();
        Ok(())
    }

    fn place(&mut self, letter: char) -> Result<(), HopError> {
        let piece = make_piece(letter, self.game_type, &mut self.counts)?;
        if self.placed_any {
            self.position = self.position.to(self.heading);
        }
        self.record(piece, self.position);
        self.placed_any = true;
        Ok(())
    }

    fn chain_ref(&mut self, n: usize, chars: &mut Peekable<Chars<'_>>) -> Result<(), HopError> {
        let (anchor, heading) = self.anchor(n)?;
        match chars.next() {
            Some('=') => {
                let letter = chars.next().ok_or(HopError::MissingStackBug)?;
                let piece = make_piece(letter, self.game_type, &mut self.counts)?;
                self.position = anchor;
                self.record(piece, anchor);
                Ok(())
            }
            Some('+') => Ok(self.branch_from(anchor, heading, HOP_PLUS)),
            Some('-') => Ok(self.branch_from(anchor, heading, HOP_MINUS)),
            _ => Err(HopError::BadChainOp(n)),
        }
    }

    fn anchor(&self, n: usize) -> Result<(Position, Direction), HopError> {
        n.checked_sub(1)
            .and_then(|i| self.current_scope().get(i))
            .copied()
            .ok_or(HopError::BadChainRef(n))
    }

    fn branch_from(&mut self, anchor: Position, heading: Direction, rotation: Rotation) {
        self.position = anchor;
        self.heading = heading.next_direction(rotation);
    }

    fn record(&mut self, piece: Piece, at: Position) {
        self.board.insert(at, piece, true);
        self.last_placed = Some((piece, at));
        let heading = self.heading;
        self.scopes
            .last_mut()
            .expect("scope stack is never empty")
            .push((at, heading));
    }

    fn current_scope(&self) -> &Vec<(Position, Direction)> {
        self.scopes.last().expect("scope stack is never empty")
    }

    fn finish(mut self) -> Result<Board, HopError> {
        if !self.placed_any {
            return Err(HopError::NoStartBug);
        }
        if self.scopes.len() != 1 {
            return Err(HopError::UnbalancedParens);
        }
        self.board.last_moved = self.marked;
        Ok(self.board)
    }
}

fn make_piece(
    letter: char,
    game_type: GameType,
    counts: &mut HashMap<(Color, Bug), usize>,
) -> Result<Piece, HopError> {
    if letter.eq_ignore_ascii_case(&'d') {
        return Err(HopError::Dragonfly);
    }
    let color = if letter.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    let bug = Bug::from_str(&letter.to_string()).map_err(|_| HopError::BadChar(letter))?;
    let max = bug.count(game_type);
    if max == 0 {
        return Err(HopError::PieceNotInGameType { bug, game_type });
    }
    let count = counts.entry((color, bug)).or_insert(0);
    *count += 1;
    if *count > max {
        return Err(HopError::TooManyPieces { color, bug });
    }
    let order = if bug.has_order() { *count } else { 0 };
    Ok(Piece::new_from(bug, color, order))
}

fn read_number(first: char, chars: &mut Peekable<Chars<'_>>) -> Result<usize, HopError> {
    let mut text = first.to_string();
    let mut n = first
        .to_digit(10)
        .expect("caller only calls this on an ascii digit") as usize;
    while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
        let c = chars.next().expect("just peeked");
        text.push(c);
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add(d as usize))
            .ok_or_else(|| HopError::NumberTooLarge(text.clone()))?;
    }
    Ok(n)
}
