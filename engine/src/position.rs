use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{
    board::{Board, BOARD_SIZE},
    direction::Direction,
    game_error::GameError,
    piece::Piece,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Position {
    pub q: u8,
    pub r: u8,
    pub l: u8,
}

impl Position {
    pub fn new(q: u8, r: u8, l: u8) -> Self {
        Self { q, r, l }
    }
}

#[derive(Debug)]
pub struct CircleIter {
    pub current_position: Position,
    pub direction: Direction,      // Direction that gets explored next
    pub main_direction: Direction, // in which direction of the wQ is the bQ
    pub level: usize,              // ring'th level
    pub index: Option<usize>,      // how many steps have to explored so far
    pub side_index: usize,         // at which position are we in the side
    pub revolution: Rotation,      // clockwise or counter clockwise
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(usize)]
pub enum Rotation {
    C = 0,
    CC = 1,
}

impl fmt::Display for CircleIter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}:\t {} {:<10} l:{}, s:{}",
            self.index,
            self.current_position,
            self.direction.to_string(),
            self.level,
            self.side_index
        )
    }
}

impl CircleIter {
    pub fn new(origin: Position, direction: Direction, revolution: Rotation) -> Self {
        CircleIter {
            revolution,
            current_position: origin,
            main_direction: direction,
            direction,
            level: 0,
            side_index: 0,
            index: None,
        }
    }
}

impl Iterator for CircleIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        // step into next ring
        if self.index.is_none() {
            self.index = Some(0);
            return Some(self.current_position);
        }
        let index = self.index.unwrap();
        self.index = Some(index + 1);
        if index == 3 * (self.level * self.level + self.level) {
            self.level += 1;
            self.current_position = self.current_position.to(self.direction);
            if index != 0 {
                self.current_position = self.current_position.to(self.main_direction);
            }
            self.direction = self
                .main_direction
                .next_direction(self.revolution)
                .next_direction(self.revolution);
            self.side_index = 0;
            return Some(self.current_position);
        }
        // steping level-times in self.current_direction
        self.current_position = self.current_position.to(self.direction);
        if self.side_index == self.level - 1 {
            self.side_index = 0;
            self.direction = self.direction.next_direction(self.revolution);
            //direction = direction.next_direction(self.revolution);
        } else {
            self.side_index += 1;
        }
        Some(self.current_position)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "q:{}, r:{}", self.q, self.r)
    }
}

impl Position {
    pub fn new(q: i32, r: i32) -> Self {
        let q = q.rem_euclid(BOARD_SIZE);
        let r = r.rem_euclid(BOARD_SIZE);
        Self { q, r }
    }

    pub fn initial_spawn_position() -> Self {
        Self { q: 16, r: 16 }
    }

    fn wrap_around(num: i32) -> i32 {
        if num == (BOARD_SIZE - 1) {
            return -1;
        }
        if num == -(BOARD_SIZE - 1) {
            return 1;
        }
        num
    }

    pub fn is_neighbor(&self, to: Position) -> bool {
        let diff = (
            Self::wrap_around(to.q - self.q),
            Self::wrap_around(to.r - self.r),
        );
        matches!(
            diff,
            (0, -1) | (0, 1) | (1, -1) | (-1, 1) | (-1, 0) | (1, 0)
        )
    }

    // this implements "odd-r horizontal" which offsets odd rows to the right
    pub fn direction(&self, to: Position) -> Direction {
        let diff = (
            Self::wrap_around(to.q - self.q),
            Self::wrap_around(to.r - self.r),
        );
        match diff {
            (0, -1) => Direction::NW,
            (0, 1) => Direction::SE,

            (1, -1) => Direction::NE,
            (-1, 1) => Direction::SW,

            (-1, 0) => Direction::W,
            (1, 0) => Direction::E,
            // This panic is okay, because if it ever gets called with an invalid move, it
            // implies there is a problem with the engine itself, not with user input
            (q, r) => {
                panic!("(odd) Direction of movement unknown, from: {self} to: {to} ({q},{r})")
            }
        }
    }

    pub fn common_adjacent_positions(&self, to: Position) -> (Position, Position) {
        let (dir1, dir2) = self.direction(to).adjacent_directions();
        (self.to(dir1), self.to(dir2))
    }

    pub fn positions_around(&self) -> impl Iterator<Item = Position> {
        [
            Position::new(self.q, self.r - 1),     // NW
            Position::new(self.q, self.r + 1),     //SE
            Position::new(self.q + 1, self.r - 1), // NE
            Position::new(self.q - 1, self.r + 1), // SW
            Position::new(self.q - 1, self.r),     // W
            Position::new(self.q + 1, self.r),     // E
        ]
        .into_iter()
    }

    pub fn to(&self, direction: Direction) -> Position {
        match direction {
            Direction::NW => Position::new(self.q, self.r - 1),
            Direction::SE => Position::new(self.q, self.r + 1),
            Direction::NE => Position::new(self.q + 1, self.r - 1),
            Direction::SW => Position::new(self.q - 1, self.r + 1),
            Direction::W => Position::new(self.q - 1, self.r),
            Direction::E => Position::new(self.q + 1, self.r),
        }
    }

    pub fn from_string(s: &str, board: &Board) -> Result<Position, GameError> {
        if s.starts_with('.') || s.is_empty() {
            return Ok(Position::initial_spawn_position());
        }

        lazy_static! {
            static ref RE: Regex = Regex::new(r"([-/\\]?)([wb][ABGMLPSQ]\d?)([-/\\]?)")
                .expect("This regex should compile");
        }
        if let Some(cap) = RE.captures(s) {
            let piece: Piece = cap[2].parse()?;
            if let Some(mut position) = board.position_of_piece(piece) {
                if !cap[1].is_empty() {
                    match &cap[1] {
                        "\\" => {
                            position = position.to(Direction::NW);
                        }
                        "-" => {
                            position = position.to(Direction::W);
                        }
                        "/" => {
                            position = position.to(Direction::SW);
                        }
                        any => {
                            return Err(GameError::InvalidDirection {
                                direction: any.to_string(),
                            })
                        }
                    }
                }
                if !cap[3].is_empty() {
                    match &cap[3] {
                        "/" => {
                            position = position.to(Direction::NE);
                        }
                        "-" => {
                            position = position.to(Direction::E);
                        }
                        "\\" => {
                            position = position.to(Direction::SE);
                        }
                        any => {
                            return Err(GameError::InvalidDirection {
                                direction: any.to_string(),
                            })
                        }
                    }
                }
                return Ok(position);
            }
        }
        Err(GameError::ParsingError {
            found: s.to_string(),
            typ: "position".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_direction_and_to() {
        for position in [Position::new(0, 0), Position::new(0, 1)] {
            for direction in Direction::all() {
                let new_position = position.to(direction);
                let opposite_direction = new_position.direction(position);
                let initial_position = new_position.to(opposite_direction);
                assert_eq!(position, initial_position);
            }
        }
    }

    #[test]
    fn tests_direction_and_to_circles() {
        use Direction::*;
        let pos = Position::new(0, 0);
        let nw = pos.to(Direction::NW);
        let sw = nw.to(Direction::SW);
        let e = sw.to(Direction::E);
        assert_eq!(e, pos);
        let dirs = vec![NW, NW, SW, SW, E, E];
        let pos_0_0 = Position::new(0, 0);
        let mut pos = Position::new(0, 0);
        for direction in dirs {
            pos = pos.to(direction)
        }
        assert_eq!(pos_0_0, pos);
        let dirs = vec![NW, SW, SE, E, NE, NW, SW];
        let pos_0_0 = Position::new(0, 0);
        let mut pos = Position::new(0, 0);
        for direction in dirs {
            pos = pos.to(direction)
        }
        assert_eq!(pos_0_0, pos);
    }

    #[test]
    fn tests_step() {
        let origin = Position::new(16, 16);
        use Direction::*;
        let direction = Direction::NW;
        let mut ci = CircleIter::new(origin, direction, Rotation::C).enumerate();
        let positions = [
            origin,
            origin.to(NW),
            origin.to(NE),
            origin.to(E),
            origin.to(SE),
            origin.to(SW),
            origin.to(W),
            origin.to(NW).to(NW),
            origin.to(NW).to(NE),
            origin.to(NE).to(NE),
            origin.to(NE).to(E),
            origin.to(E).to(E),
            origin.to(E).to(SE),
            origin.to(SE).to(SE),
            origin.to(SE).to(SW),
            origin.to(SW).to(SW),
            origin.to(SW).to(W),
            origin.to(W).to(W),
            origin.to(W).to(NW),
            origin.to(NW).to(NW).to(NW),
            origin.to(NW).to(NW).to(NW).to(E),
            origin.to(NW).to(NW).to(NW).to(E).to(E),
            origin.to(NE).to(NE).to(NE),
        ];
        for should_be in positions.into_iter().enumerate() {
            let is = ci.next().unwrap();
            assert_eq!(should_be, is);
        }
        let mut ci = CircleIter::new(origin, direction, Rotation::CC).enumerate();
        let positions = [
            origin,
            origin.to(NW),
            origin.to(W),
            origin.to(SW),
            origin.to(SE),
            origin.to(E),
            origin.to(NE),
            origin.to(NW).to(NW),
        ];
        for should_be in positions.into_iter().enumerate() {
            let is = ci.next().unwrap();
            assert_eq!(should_be, is);
        }
    }
}
