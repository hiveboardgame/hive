use crate::board::MidMoveBoard;
use crate::{
    board::Board, game_error::GameError, game_type::GameType, position::Position,
    torus_array::TorusArray,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{collections::HashSet, fmt, str::FromStr};

#[derive(Hash, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum Bug {
    Ant = 0,
    Beetle = 1,
    Grasshopper = 2,
    Ladybug = 3,
    Mosquito = 4,
    Pillbug = 5,
    Queen = 6,
    Spider = 7,
}

impl From<Bug> for u8 {
    fn from(bug: Bug) -> Self {
        bug as u8
    }
}

impl From<u8> for Bug {
    fn from(item: u8) -> Self {
        match item {
            0 => Bug::Ant,
            1 => Bug::Beetle,
            2 => Bug::Grasshopper,
            3 => Bug::Ladybug,
            4 => Bug::Mosquito,
            5 => Bug::Pillbug,
            6 => Bug::Queen,
            7 => Bug::Spider,
            _ => panic!(),
        }
    }
}

impl fmt::Display for Bug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Bug {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" | "a" => Ok(Bug::Ant),
            "B" | "b" => Ok(Bug::Beetle),
            "G" | "g" => Ok(Bug::Grasshopper),
            "L" | "l" => Ok(Bug::Ladybug),
            "M" | "m" => Ok(Bug::Mosquito),
            "P" | "p" => Ok(Bug::Pillbug),
            "Q" | "q" => Ok(Bug::Queen),
            "S" | "s" => Ok(Bug::Spider),
            any => Err(GameError::ParsingError {
                found: any.to_string(),
                typ: "bug string".to_string(),
            }),
        }
    }
}

impl Bug {
    pub fn all() -> impl Iterator<Item = Bug> {
        [
            Bug::Ant,
            Bug::Beetle,
            Bug::Grasshopper,
            Bug::Ladybug,
            Bug::Mosquito,
            Bug::Pillbug,
            Bug::Queen,
            Bug::Spider,
        ]
        .into_iter()
    }

    pub fn as_str(&self) -> &'static str {
        &self.name()[0..=0]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Bug::Ant => "Ant",
            Bug::Beetle => "Beetle",
            Bug::Grasshopper => "Grasshopper",
            Bug::Ladybug => "Ladybug",
            Bug::Mosquito => "Mosquito",
            Bug::Pillbug => "Pillbug",
            Bug::Queen => "Queen",
            Bug::Spider => "Spider",
        }
    }

    pub fn as_emoji(&self) -> String {
        match self {
            Bug::Ant => '\u{1f41c}',
            Bug::Beetle => '\u{1fab2}',
            Bug::Grasshopper => '\u{1f997}',
            Bug::Ladybug => '\u{1f41e}',
            Bug::Mosquito => '\u{1f99f}',
            Bug::Pillbug => '\u{1f48a}',
            Bug::Queen => '\u{1f41d}',
            Bug::Spider => '\u{1f577}',
        }
        .clone()
        .to_string()
    }

    pub fn bugs_count(game_type: GameType) -> HashMap<Bug, i8> {
        let mut bugs = HashMap::new();
        bugs.insert(Bug::Ant, 3);
        bugs.insert(Bug::Beetle, 2);
        bugs.insert(Bug::Grasshopper, 3);
        bugs.insert(Bug::Queen, 1);
        bugs.insert(Bug::Spider, 2);
        match game_type {
            GameType::Base => {}
            GameType::M => {
                bugs.insert(Bug::Mosquito, 1);
            }
            GameType::L => {
                bugs.insert(Bug::Ladybug, 1);
            }
            GameType::P => {
                bugs.insert(Bug::Pillbug, 1);
            }
            GameType::ML => {
                bugs.insert(Bug::Mosquito, 1);
                bugs.insert(Bug::Ladybug, 1);
            }
            GameType::MP => {
                bugs.insert(Bug::Mosquito, 1);
                bugs.insert(Bug::Pillbug, 1);
            }
            GameType::LP => {
                bugs.insert(Bug::Ladybug, 1);
                bugs.insert(Bug::Pillbug, 1);
            }
            GameType::MLP => {
                bugs.insert(Bug::Mosquito, 1);
                bugs.insert(Bug::Ladybug, 1);
                bugs.insert(Bug::Pillbug, 1);
            }
        }
        bugs
    }

    pub fn has_order(&self) -> bool {
        match self {
            Bug::Ant | Bug::Beetle | Bug::Grasshopper | Bug::Spider => true,
            Bug::Ladybug | Bug::Mosquito | Bug::Pillbug | Bug::Queen => false,
        }
    }

    pub fn available_moves(position: Position, board: &Board) -> HashMap<Position, Vec<Position>> {
        let mut moves = HashMap::default();
        if !board.is_pinned(
            board
                .top_piece(position)
                .expect("There must be something at this position"),
        ) {
            let positions = match board.top_bug(position) {
                Some(Bug::Ant) => Bug::ant_moves(position, board),
                Some(Bug::Beetle) => Bug::beetle_moves(position, board),
                Some(Bug::Grasshopper) => Bug::grasshopper_moves(position, board),
                Some(Bug::Ladybug) => Bug::ladybug_moves(position, board),
                Some(Bug::Mosquito) => Bug::mosquito_moves(position, board),
                Some(Bug::Pillbug) => Bug::pillbug_moves(position, board).collect(),
                Some(Bug::Queen) => Bug::queen_moves(position, board).collect(),
                Some(Bug::Spider) => Bug::spider_moves(position, board),
                None => Vec::new(),
            };
            moves.insert(position, positions);
        }
        moves.extend(Bug::available_abilities(position, board));
        moves
    }

    pub fn available_abilities(
        position: Position,
        board: &Board,
    ) -> HashMap<Position, Vec<Position>> {
        match board.top_bug(position) {
            Some(Bug::Pillbug) => Bug::pillbug_throw(position, board),
            Some(Bug::Mosquito)
                if board.level(position) == 1 && board.neighbor_is_a(position, Bug::Pillbug) =>
            {
                Bug::pillbug_throw(position, board)
            }
            _ => HashMap::default(),
        }
    }

    fn crawl_negative_space<'a>(
        position: Position,
        board: &'a MidMoveBoard<'a>,
    ) -> impl Iterator<Item = Position> + 'a {
        position
            .positions_around()
            .filter(|pos| board.is_negative_space(*pos))
            .filter(move |pos| !board.gated(1, position, *pos))
    }

    fn crawl(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        board.positions_taken_around(position).flat_map(move |pos| {
            let mut positions = vec![];
            let (pos1, pos2) = position.common_adjacent_positions(pos);
            if !board.gated(1, position, pos1) && !board.occupied(pos1) {
                positions.push(pos1);
            }
            if !board.gated(1, position, pos2) && !board.occupied(pos2) {
                positions.push(pos2);
            }
            positions
        })
    }

    fn climb(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        board
            .positions_taken_around(position)
            .filter(move |pos| !board.gated(board.level(*pos) + 1, position, *pos))
    }

    fn descend(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        position.positions_around().filter(move |pos| {
            board.level(*pos) < board.level(position)
                && !board.gated(board.level(position), position, *pos)
        })
    }

    fn ant_moves(position: Position, board: &Board) -> Vec<Position> {
        //                               found  explored
        let mut state = TorusArray::new((false, false));
        state.set(position, (true, true));
        let board = MidMoveBoard::new(board, board.top_piece(position).unwrap(), position);
        let mut found_pos = Vec::with_capacity(24);
        let mut unexplored = Vec::with_capacity(24);
        unexplored.push(position);
        Bug::ant_rec(&mut state, &mut found_pos, &mut unexplored, &board);
        found_pos
    }

    fn ant_rec(
        state: &mut TorusArray<(bool, bool)>,
        found_pos: &mut Vec<Position>,
        unexplored: &mut Vec<Position>,
        board: &MidMoveBoard,
    ) {
        while let Some(position) = unexplored.pop() {
            let (found, explored) = state.get(position);
            if !found {
                state.set(position, (true, *explored));
                found_pos.push(position);
            }
            for pos in Bug::crawl_negative_space(position, board) {
                let (found, explored) = state.get(pos);
                if !explored && !found && board.is_negative_space(pos) {
                    state.set(pos, (*found, true));
                    unexplored.push(pos);
                }
            }
        }
    }

    pub fn beetle_moves(position: Position, board: &Board) -> Vec<Position> {
        let mut positions = Vec::new();
        for pos in Bug::climb(position, board) {
            positions.push(pos);
        }
        if board.level(position) == 1 {
            for pos in Bug::crawl(position, board) {
                if !positions.contains(&pos) {
                    positions.push(pos);
                }
            }
        } else {
            for pos in Bug::descend(position, board) {
                if !positions.contains(&pos) {
                    positions.push(pos);
                }
            }
        }
        positions
    }

    pub fn grasshopper_moves(position: Position, board: &Board) -> Vec<Position> {
        // get the directions of the grasshopper's neighbors
        let mut positions = vec![];
        // move in the given direction
        for dir in board
            .positions_taken_around(position)
            .map(|pos| position.direction(pos))
        {
            let mut cur_pos = position;
            // until there is a free position
            while board.occupied(cur_pos.to(dir)) {
                cur_pos = cur_pos.to(dir);
            }
            // then add the free position
            positions.push(cur_pos.to(dir));
        }
        positions
    }

    fn ladybug_moves(position: Position, board: &Board) -> Vec<Position> {
        // find all adjacent bugs to climb on
        let first = Bug::climb(position, board);
        // stay on top of the hive by performing another climb
        let second: HashSet<Position> = first
            .flat_map(|first_pos| {
                Bug::climb(first_pos, board)
                    .filter(|pos| *pos != position && *pos != first_pos)
                    .collect::<HashSet<Position>>()
            })
            .collect::<HashSet<Position>>();
        // then find available and ungated positions
        let third: HashSet<Position> = second
            .iter()
            .flat_map(|pos| {
                board
                    .positions_available_around(*pos)
                    .filter(|p| !board.gated(board.level(*pos) + 1, *pos, *p) && *p != position)
                    .collect::<HashSet<Position>>()
            })
            .collect::<HashSet<Position>>();
        return third.iter().cloned().collect();
    }

    fn mosquito_moves(position: Position, board: &Board) -> Vec<Position> {
        return if board.level(position) == 1 {
            board
                .neighbors(position)
                .flat_map(|pieces| {
                    match pieces.top_piece().expect("Could not get last piece").bug() {
                        Bug::Ant => Bug::ant_moves(position, board),
                        Bug::Beetle => Bug::beetle_moves(position, board),
                        Bug::Grasshopper => Bug::grasshopper_moves(position, board),
                        Bug::Ladybug => Bug::ladybug_moves(position, board),
                        Bug::Mosquito => vec![],
                        Bug::Pillbug => Bug::pillbug_moves(position, board).collect(),
                        Bug::Queen => Bug::queen_moves(position, board).collect(),
                        Bug::Spider => Bug::spider_moves(position, board),
                    }
                })
                .collect()
        } else {
            Bug::beetle_moves(position, board)
        };
    }

    fn pillbug_moves(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        Bug::crawl(position, board)
    }

    fn pillbug_throw(position: Position, board: &Board) -> HashMap<Position, Vec<Position>> {
        let mut moves = HashMap::default();
        // get all the positions the pillbug can throw a bug to
        let to = board
            .positions_available_around(position)
            .filter(|pos| !board.gated(2, position, *pos))
            .collect::<Vec<Position>>();
        // get bugs around the pillbug that aren't pinned
        for pos in board.positions_taken_around(position).filter(|p| {
            !board.is_pinned(board.top_piece(*p).unwrap())
                && !board.gated(2, *p, position)
                && board.level(*p) <= 1
        }) {
            moves.insert(pos, to.clone());
        }
        moves
    }

    fn queen_moves(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        Bug::crawl(position, board)
    }

    fn spider_moves(position: Position, board: &Board) -> Vec<Position> {
        let board = MidMoveBoard::new(board, board.top_piece(position).unwrap(), position);
        let mut res = Vec::new();
        for pos1 in Bug::crawl_negative_space(position, &board) {
            for pos2 in Bug::crawl_negative_space(pos1, &board).filter(move |pos| *pos != position)
            {
                for pos3 in Bug::crawl_negative_space(pos2, &board).filter(move |pos| *pos != pos1)
                {
                    if pos3 != position {
                        res.push(pos3);
                    }
                }
            }
        }
        res.sort();
        res.dedup();
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{color::Color, piece::Piece};

    #[test]
    fn tests_available_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
        );
        let moves = Bug::available_moves(Position::new(0, 0), &board);
        assert_eq!(moves.get(&Position::new(0, 0)).unwrap().len(), 2);
        let moves = Bug::available_moves(Position::new(1, 0), &board);
        assert_eq!(moves.get(&Position::new(1, 0)).unwrap().len(), 6);
    }

    #[test]
    fn tests_available_abilities() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        let positions = Bug::available_abilities(Position::new(0, 0), &board);
        assert_eq!(positions.get(&Position::new(1, 0)).unwrap().len(), 5);
        let positions = Bug::available_abilities(Position::new(1, 0), &board);
        assert_eq!(positions.get(&Position::new(0, 0)).unwrap().len(), 5);
    }

    #[test]
    fn tests_pillbug_throw() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        let positions = Bug::pillbug_throw(Position::new(0, 0), &board);
        assert_eq!(positions.get(&Position::new(1, 0)).unwrap().len(), 5);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
        );
        let positions = Bug::pillbug_throw(Position::new(0, 0), &board);
        assert!(!positions.contains_key(&Position::new(1, 0)));
    }

    #[test]
    fn tests_pillbug_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        let positions = Bug::pillbug_moves(Position::new(0, 0), &board);
        assert_eq!(positions.count(), 2);
    }

    #[test]
    fn tests_mosquito_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 0);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 5);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 2);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 6);
    }

    #[test]
    fn tests_descend() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::NE),
            Piece::new_from(Bug::Ant, Color::White, 1),
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::NE),
            Piece::new_from(Bug::Beetle, Color::Black, 2),
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::SE),
            Piece::new_from(Bug::Ant, Color::White, 2),
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::SE),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        let positions = Bug::descend(Position::new(0, 0), &board);
        assert_eq!(positions.count(), 3);
        let mut positions = Bug::descend(Position::new(0, 0), &board);
        assert!(positions.any(|pos| pos == Position::new(0, 0).to(crate::direction::Direction::SW)));
        let mut positions = Bug::descend(Position::new(0, 0), &board);
        assert!(positions.any(|pos| pos == Position::new(0, 0).to(crate::direction::Direction::W)));
        let mut positions = Bug::descend(Position::new(0, 0), &board);
        assert!(positions.any(|pos| pos == Position::new(0, 0).to(crate::direction::Direction::NW)));
    }

    #[test]
    fn tests_climb() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
        );
        let positions = Bug::climb(Position::new(1, 0), &board);
        assert_eq!(positions.count(), 1);
        let mut positions = Bug::climb(Position::new(1, 0), &board);
        assert!(positions.any(|pos| pos == Position::new(0, 0)));

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
            let positions = Bug::climb(Position::new(0, 0), &board);
            assert_eq!(positions.count(), i + 1);
        }

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
        }
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::NE),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::SE),
            Piece::new_from(Bug::Beetle, Color::Black, 2),
        );
        let positions = Bug::climb(Position::new(0, 0), &board);
        assert_eq!(positions.count(), 5);
    }

    #[test]
    fn tests_crawl() {
        use crate::direction::Direction;
        // one neighbor gives 2 positions to move to
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NE)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::SE)));

        // just a quick sanity check
        let mut board = Board::new();
        board.insert(
            Position::new(0, 1),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        for (i, pos) in Position::new(0, 1).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
            let positions = Bug::crawl(Position::new(0, 1), &board).collect::<Vec<Position>>();
            assert_eq!(positions.len(), 2);
            board.remove(pos);
        }

        // two adjacent neighbors means two positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
        );
        board.insert(
            Position::new(0, 1),
            Piece::new_from(Bug::Ant, Color::White, 1),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NE)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::SW)));

        // two (opposite) neighbors give 4 positions to move to
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 4);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NE)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::SE)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::SW)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NW)));

        // two neighbors that form a gate
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(-1, 1),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NE)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::W)));

        // a third neighbor forms a gate so we are back to 2 positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Ant, Color::Black, 3),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 1)));
        assert!(positions.contains(&Position::new(-1, 1)));

        // three neighbors that form a tripple gate means no movement
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 1),
        );
        board.insert(
            Position::new(0, 0).to(Direction::NE),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(0, 0).to(Direction::SE),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        board.insert(
            Position::new(0, 0).to(Direction::W),
            Piece::new_from(Bug::Ant, Color::Black, 3),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 0);

        // three neighbors no gate -> 2 positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 1),
        );
        board.insert(
            Position::new(0, 0).to(Direction::NE),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(0, 0).to(Direction::E),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        board.insert(
            Position::new(0, 0).to(Direction::SE),
            Piece::new_from(Bug::Ant, Color::Black, 3),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::SW)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NW)));

        // four neighbors no gate -> 2 positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 1),
        );
        board.insert(
            Position::new(0, 0).to(Direction::NE),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(0, 0).to(Direction::E),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        board.insert(
            Position::new(0, 0).to(Direction::SE),
            Piece::new_from(Bug::Ant, Color::Black, 3),
        );
        board.insert(
            Position::new(0, 0).to(Direction::SW),
            Piece::new_from(Bug::Ladybug, Color::Black, 1),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::W)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NW)));

        // five neighbors -> 0 positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Ant, Color::Black, 2),
        );
        board.insert(
            Position::new(0, 1),
            Piece::new_from(Bug::Ant, Color::Black, 3),
        );
        board.insert(
            Position::new(-1, 1),
            Piece::new_from(Bug::Ladybug, Color::Black, 0),
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn tests_queen_moves() {
        tests_crawl()
    }

    #[test]
    fn tests_spider_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Spider, Color::White, 1),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        let positions = Bug::spider_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 1);
        assert!(positions.contains(&Position::new(2, 0)));
    }

    #[test]
    fn tests_ladybug_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(-2, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        assert_eq!(Bug::ladybug_moves(Position::new(0, 0), &board).len(), 5);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
        }
        board.remove(Position::new(1, 0));
        assert_eq!(Bug::ladybug_moves(Position::new(0, 0), &board).len(), 12);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
        }
        board.insert(
            Position::new(-2, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
        );
        board.remove(Position::new(1, 0));
        assert_eq!(Bug::ladybug_moves(Position::new(0, 0), &board).len(), 14);
    }

    #[test]
    fn tests_beetle_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Queen, Color::White, 0),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
        );
        assert_eq!(Bug::beetle_moves(Position::new(0, 0), &board).len(), 4);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
        }
        assert_eq!(Bug::beetle_moves(Position::new(0, 0), &board).len(), 6);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
            );
        }
        board.remove(Position::new(1, 0));
        assert_eq!(Bug::beetle_moves(Position::new(0, 0), &board).len(), 5);
    }

    #[test]
    fn tests_ant_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ant, Color::White, 1),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
        );
        assert_eq!(
            board
                .neighbors(Position::new(0, 0))
                .last()
                .unwrap()
                .top_piece()
                .unwrap(),
            Piece::new_from(Bug::Beetle, Color::White, 1)
        );
        assert_eq!(Bug::ant_moves(Position::new(0, 0), &board).len(), 5);
    }

    #[test]
    fn tests_grasshopper_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Grasshopper, Color::White, 1),
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Ant, Color::from((i % 2) as u8), i / 2 + 1),
            );
        }
        assert_eq!(Bug::grasshopper_moves(Position::new(0, 0), &board).len(), 6);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Grasshopper, Color::White, 1),
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
        );
        assert_eq!(Bug::grasshopper_moves(Position::new(0, 0), &board).len(), 1);
        assert_eq!(
            *Bug::grasshopper_moves(Position::new(0, 0), &board)
                .last()
                .unwrap(),
            Position::new(2, 0)
        );

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Grasshopper, Color::White, 1),
        );
        assert_eq!(Bug::grasshopper_moves(Position::new(0, 0), &board).len(), 0);
    }
}
