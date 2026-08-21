use std::{
    collections::HashMap,
    fmt,
    iter::Peekable,
    str::{Chars, FromStr},
};

use crate::{
    board::{Board, BOARD_SIZE},
    bug::Bug,
    canonical_hash::{axis_origin, canonical_hash},
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

/// One ground piece plus every climber the game can field: two Beetles and one Mosquito a side.
const MAX_STACK: usize = 7;

#[derive(Debug, Clone)]
pub struct HopPosition {
    pub board: Board,
    pub game_type: GameType,
    pub to_move: Color,
    pub orientation: Orientation,
}

/// HOP's trailing display hint, which we carry but never act on: `N` rotates the drawing N
/// sixths clockwise, `m` mirrors it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Orientation {
    pub rotation: Option<u8>,
    pub mirrored: bool,
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(rotation) = self.rotation {
            write!(f, "{rotation}")?;
        }
        if self.mirrored {
            write!(f, "m")?;
        }
        Ok(())
    }
}

/// A lone piece cannot belong to the side to move - nobody has moved yet, so the other player
/// is up. Either colour may have opened: a pasted position is not a played game.
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
    let (to_move, orientation) = parse_player(player)?;
    let mut board = parse_topology(topology, game_type)?;
    derive_stun(&mut board, to_move);
    // A lone piece was just placed, so its owner cannot also be the side to move.
    if board.played == 1 {
        let position = board
            .all_taken_positions()
            .next()
            .expect("played == 1 implies one taken position");
        let piece_color = board
            .top_piece(position)
            .expect("position is taken")
            .color();
        if piece_color == to_move {
            return Err(HopError::LonePieceOwnerToMove);
        }
    }
    queens_are_accounted_for(&board)?;
    Ok(HopPosition {
        board,
        game_type,
        to_move,
        orientation,
    })
}

/// A placement can only stun through Black's first reply (hence `played <= 2`), because before
/// that White's lone piece cannot be both the Pillbug and the Queen needed to move it.
fn derive_stun(board: &mut Board, to_move: Color) {
    if board.played <= 2 {
        return;
    }
    if let Some((piece, at)) = board.last_moved {
        board.set_stunned(at, piece, false, to_move.opposite_color());
    }
}

/// Anything stacked moved there itself, which needs its Queen down. Reachability is checked
/// when a state is built, not here: HOP stays a format reader.
fn queens_are_accounted_for(board: &Board) -> Result<(), HopError> {
    let queen_played =
        |color: Color| board.piece_already_played(Piece::new_from(Bug::Queen, color, 0));

    for position in board.all_taken_positions() {
        let stack = board.board.get(position);
        for level in 1..stack.size as usize {
            let color = stack.pieces[level].color();
            if !queen_played(color) {
                return Err(HopError::QueenRequiredToMove { color });
            }
        }
    }
    Ok(())
}

/// Canonical hash of a HOP string. No stun argument: the parser recovers it from `!`, and no
/// caller knows better - as a parameter it was silently dropped for one position in seven.
pub fn to_hash(hop: &str) -> Result<i64, HopError> {
    let parsed = parse(hop)?;
    Ok(canonical_hash(&parsed.board, parsed.to_move, parsed.board.stunned) as i64)
}

/// Judged by the inventory the modifiers produce, not the spelling: `base+1M1m` and
/// `ultimate-Pp` name existing types. Anything else is Pepeke's unequal games, and refused.
fn parse_game_type(field: &str) -> Result<GameType, HopError> {
    let field = field.trim();
    // `d` anywhere means the Dragonfly, which earns a clearer error than "unsupported".
    if field.contains(['d', 'D']) {
        return Err(HopError::Dragonfly);
    }
    // An omitted game type is the two-field form, which is what we emit for `ultimate`.
    if field.is_empty() {
        return Ok(GameType::MLP);
    }
    let unsupported = || HopError::UnsupportedGameType(field.to_string());
    let modifiers_at = field.find(['+', '-']).unwrap_or(field.len());
    let (name, modifiers) = field.split_at(modifiers_at);
    // Per-side inventory, indexed [side][bug]; uppercase letters are White's, lowercase Black's.
    let mut inventory = [[0i64; 8]; 2];
    if name.eq_ignore_ascii_case("base") || name.eq_ignore_ascii_case("ultimate") {
        let start = if name.eq_ignore_ascii_case("base") {
            GameType::Base
        } else {
            GameType::MLP
        };
        for bug in Bug::all() {
            inventory[0][bug as usize] = bug.count(start) as i64;
            inventory[1][bug as usize] = bug.count(start) as i64;
        }
    } else if !add_tokens(name, 1, &mut inventory, &unsupported)? {
        return Err(unsupported());
    }

    let mut chars = modifiers.chars().peekable();
    while let Some(sign) = chars.next() {
        let sign: i64 = match sign {
            '+' => 1,
            '-' => -1,
            _ => return Err(unsupported()),
        };
        let group: String =
            std::iter::from_fn(|| chars.next_if(|c| !matches!(c, '+' | '-'))).collect();
        if !add_tokens(&group, sign, &mut inventory, &unsupported)? {
            return Err(unsupported());
        }
    }

    [
        GameType::Base,
        GameType::M,
        GameType::L,
        GameType::P,
        GameType::ML,
        GameType::LP,
        GameType::MP,
        GameType::MLP,
    ]
    .into_iter()
    .find(|&candidate| {
        Bug::all().into_iter().all(|bug| {
            let expected = bug.count(candidate) as i64;
            inventory[0][bug as usize] == expected && inventory[1][bug as usize] == expected
        })
    })
    .ok_or_else(unsupported)
}

/// Tokens are `[count]letter`, so the same inventory can be spelled many ways; only the totals
/// decide.
fn add_tokens(
    group: &str,
    sign: i64,
    inventory: &mut [[i64; 8]; 2],
    unsupported: &impl Fn() -> HopError,
) -> Result<bool, HopError> {
    let mut chars = group.chars().peekable();
    let mut applied = false;
    while chars.peek().is_some() {
        let mut count: i64 = 0;
        let mut has_digits = false;
        while let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
            chars.next();
            has_digits = true;
            count = count
                .checked_mul(10)
                .and_then(|count| count.checked_add(digit as i64))
                .ok_or_else(unsupported)?;
        }
        let count = if has_digits { count } else { 1 };
        let letter = chars.next().ok_or_else(unsupported)?;
        let bug = Bug::from_str(&letter.to_string()).map_err(|_| unsupported())?;
        let side = usize::from(letter.is_ascii_lowercase());
        inventory[side][bug as usize] = inventory[side][bug as usize]
            .checked_add(sign * count)
            .ok_or_else(unsupported)?;
        applied = true;
    }
    Ok(applied)
}

fn parse_player(field: &str) -> Result<(Color, Orientation), HopError> {
    let field = field.trim();
    let mut chars = field.chars();
    let color = match chars.next() {
        Some('w' | 'W') => Color::White,
        Some('b' | 'B') => Color::Black,
        _ => return Err(HopError::BadPlayer(field.to_string())),
    };
    let mut orientation = Orientation::default();
    for ch in chars {
        match ch {
            '0'..='5' if orientation.rotation.is_none() && !orientation.mirrored => {
                orientation.rotation = Some(ch as u8 - b'0');
            }
            'm' | 'M' if !orientation.mirrored => orientation.mirrored = true,
            _ => return Err(HopError::BadPlayer(field.to_string())),
        }
    }
    Ok((color, orientation))
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
            '!' => walk.mark_last_moved()?,
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
    /// A branch that never places leaves the walk pointing nowhere.
    pending_branch: bool,
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
            pending_branch: false,
            last_placed: None,
            marked: None,
        }
    }

    fn turn(&mut self, rotation: Rotation) {
        self.heading = self.heading.next_direction(rotation);
    }

    fn mark_last_moved(&mut self) -> Result<(), HopError> {
        self.marked = Some(self.last_placed.ok_or(HopError::MisplacedMark)?);
        Ok(())
    }

    fn open_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    fn close_scope(&mut self) -> Result<(), HopError> {
        if self.scopes.len() == 1 {
            return Err(HopError::UnbalancedParens);
        }
        if self.scopes.last().is_some_and(Vec::is_empty) {
            return Err(HopError::EmptyScope);
        }
        self.scopes.pop();
        Ok(())
    }

    fn place(&mut self, letter: char) -> Result<(), HopError> {
        let piece = make_piece(letter, self.game_type, &mut self.counts)?;
        if self.placed_any {
            self.position = self.position.to(self.heading);
        }
        self.record(piece, self.position)?;
        self.placed_any = true;
        self.pending_branch = false;
        Ok(())
    }

    fn chain_ref(&mut self, n: usize, chars: &mut Peekable<Chars<'_>>) -> Result<(), HopError> {
        let (anchor, heading) = self.anchor(n)?;
        match chars.next() {
            Some('=') => {
                let letter = chars.next().ok_or(HopError::MissingStackBug)?;
                let piece = make_piece(letter, self.game_type, &mut self.counts)?;
                self.position = anchor;
                self.record(piece, anchor)
            }
            Some('+') => {
                self.branch_from(anchor, heading, HOP_PLUS);
                Ok(())
            }
            Some('-') => {
                self.branch_from(anchor, heading, HOP_MINUS);
                Ok(())
            }
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
        self.pending_branch = true;
        self.position = anchor;
        self.heading = heading.next_direction(rotation);
    }

    /// Put `piece` on `at`, rejecting stacks Hive cannot produce: only climbers stack, and the
    /// height check is explicit because `BugStack` panics on an eighth piece.
    fn record(&mut self, piece: Piece, at: Position) -> Result<(), HopError> {
        let level = self.board.level(at);
        if level > 0 && !matches!(piece.bug(), Bug::Beetle | Bug::Mosquito) {
            return Err(HopError::CannotClimb { bug: piece.bug() });
        }
        if level >= MAX_STACK {
            return Err(HopError::StackTooTall);
        }
        self.board.insert(at, piece, true);
        self.last_placed = Some((piece, at));
        let heading = self.heading;
        self.scopes
            .last_mut()
            .expect("scope stack is never empty")
            .push((at, heading));
        Ok(())
    }

    fn current_scope(&self) -> &Vec<(Position, Direction)> {
        self.scopes.last().expect("scope stack is never empty")
    }

    fn finish(self) -> Result<Board, HopError> {
        if !self.placed_any {
            return Err(HopError::NoStartBug);
        }
        if self.pending_branch {
            return Err(HopError::EmptyBranch);
        }
        if self.scopes.len() != 1 {
            return Err(HopError::UnbalancedParens);
        }
        let translate = recentering(&self.board);
        let mut board = Board::new();
        for at in self.board.all_taken_positions() {
            let stack = self.board.board.get(at);
            for piece in &stack.pieces[..stack.size as usize] {
                board.insert(translate(at), *piece, true);
            }
        }
        board.last_moved = self.marked.map(|(piece, at)| (piece, translate(at)));
        Ok(board)
    }
}

/// The serializer's spiral can walk a long hive off the torus edge, so it comes back in pieces.
/// Unwrap each axis at its guaranteed gap, as the hash does, and centre on the spawn position.
fn recentering(board: &Board) -> impl Fn(Position) -> Position {
    let (mut q_mask, mut r_mask) = (0u32, 0u32);
    for position in board.positions.iter().flatten() {
        q_mask |= 1 << position.q;
        r_mask |= 1 << position.r;
    }
    let (q_origin, r_origin) = (axis_origin(q_mask), axis_origin(r_mask));
    let (mut q_width, mut r_width) = (0, 0);
    for position in board.positions.iter().flatten() {
        q_width = q_width.max((position.q - q_origin).rem_euclid(BOARD_SIZE));
        r_width = r_width.max((position.r - r_origin).rem_euclid(BOARD_SIZE));
    }
    let centre = Position::initial_spawn_position();
    let (q_start, r_start) = (centre.q - q_width / 2, centre.r - r_width / 2);
    move |at: Position| {
        Position::new(
            q_start + (at.q - q_origin).rem_euclid(BOARD_SIZE),
            r_start + (at.r - r_origin).rem_euclid(BOARD_SIZE),
        )
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
