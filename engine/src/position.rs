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
    pub q: i32,
    pub r: i32,
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
        Self { q: 0, r: 0 }
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
        if s.starts_with('.') {
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
}
