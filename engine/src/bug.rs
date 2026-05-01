use crate::{
    board::{Board, BOARD_SIZE},
    game_error::GameError,
    game_type::GameType,
    mid_move_board::MidMoveBoard,
    position::Position,
    torus_array::TorusArray,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, str::FromStr};

#[derive(Hash, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum Bug {
    Queen = 0,
    Ladybug = 1,
    Pillbug = 2,
    Mosquito = 3,
    Beetle = 4,
    Spider = 5,
    Grasshopper = 6,
    Ant = 7,
}

impl From<Bug> for u8 {
    fn from(bug: Bug) -> Self {
        bug as u8
    }
}

impl From<u8> for Bug {
    fn from(item: u8) -> Self {
        match item {
            0 => Bug::Queen,
            1 => Bug::Ladybug,
            2 => Bug::Pillbug,
            3 => Bug::Mosquito,
            4 => Bug::Beetle,
            5 => Bug::Spider,
            6 => Bug::Grasshopper,
            7 => Bug::Ant,
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
    pub const fn all() -> [Self; 8] {
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
    }
    // This has to be a const fn
    pub const fn into_bits(self) -> u8 {
        self as _
    }
    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => Bug::Queen,
            1 => Bug::Ladybug,
            2 => Bug::Pillbug,
            3 => Bug::Mosquito,
            4 => Bug::Beetle,
            5 => Bug::Spider,
            6 => Bug::Grasshopper,
            7 => Bug::Ant,
            _ => panic!(),
        }
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

    pub(crate) fn count(self, game_type: GameType) -> usize {
        match self {
            Bug::Ant | Bug::Grasshopper => 3,
            Bug::Beetle | Bug::Spider => 2,
            Bug::Queen => 1,
            Bug::Ladybug => usize::from(matches!(
                game_type,
                GameType::L | GameType::ML | GameType::LP | GameType::MLP
            )),
            Bug::Mosquito => usize::from(matches!(
                game_type,
                GameType::M | GameType::ML | GameType::MP | GameType::MLP
            )),
            Bug::Pillbug => usize::from(matches!(
                game_type,
                GameType::P | GameType::LP | GameType::MP | GameType::MLP
            )),
        }
    }

    pub fn has_order(&self) -> bool {
        match self {
            Bug::Ant | Bug::Beetle | Bug::Grasshopper | Bug::Spider => true,
            Bug::Ladybug | Bug::Mosquito | Bug::Pillbug | Bug::Queen => false,
        }
    }

    // Positions -> indexes in spiral iter
    //

    pub fn available_moves(position: Position, board: &Board) -> HashMap<Position, Vec<Position>> {
        let mut moves = HashMap::default();
        if !board.is_pinned(
            board
                .top_piece(position)
                .expect("There must be something at this position"),
        ) {
            moves.insert(position, Bug::normal_moves(position, board));
        }
        moves.extend(Bug::available_abilities(position, board));
        moves
    }

    pub fn normal_moves(position: Position, board: &Board) -> Vec<Position> {
        match board.top_bug(position) {
            Some(Bug::Ant) => Bug::ant_moves(position, board),
            Some(Bug::Beetle) => Bug::beetle_moves(position, board),
            Some(Bug::Grasshopper) => Bug::grasshopper_moves(position, board),
            Some(Bug::Ladybug) => Bug::ladybug_moves(position, board),
            Some(Bug::Mosquito) => Bug::mosquito_moves(position, board),
            Some(Bug::Pillbug) => Bug::pillbug_moves(position, board).collect(),
            Some(Bug::Queen) => Bug::queen_moves(position, board).collect(),
            Some(Bug::Spider) => Bug::spider_moves(position, board),
            None => Vec::new(),
        }
    }

    pub(crate) fn has_move(position: Position, board: &Board) -> bool {
        Bug::has_normal_move_matching(position, board, |_| true)
    }

    pub(crate) fn has_target_move(position: Position, target: Position, board: &Board) -> bool {
        Bug::has_normal_move_matching(position, board, |pos| pos == target)
    }

    fn has_normal_move_matching(
        position: Position,
        board: &Board,
        mut predicate: impl FnMut(Position) -> bool,
    ) -> bool {
        !Bug::scan_normal_moves_while(position, board, &mut |target| !predicate(target))
    }

    /// Scans legal normal-move targets for the top bug at `position` while
    /// `keep_scanning` returns true.
    ///
    /// Returns true only when all targets were scanned. Returns false when
    /// `keep_scanning` stopped generation early.
    fn scan_normal_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        match board.top_bug(position) {
            Some(bug) => Bug::scan_moves_as_bug_while(bug, position, board, keep_scanning),
            None => true,
        }
    }

    fn scan_moves_as_bug_while(
        bug: Bug,
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        match bug {
            Bug::Ant => Bug::scan_ant_moves_while(position, board, keep_scanning),
            Bug::Beetle => Bug::scan_beetle_moves_while(position, board, keep_scanning),
            Bug::Grasshopper => Bug::scan_grasshopper_moves_while(position, board, keep_scanning),
            Bug::Ladybug => Bug::scan_ladybug_moves_while(position, board, keep_scanning),
            Bug::Mosquito => Bug::scan_mosquito_moves_while(position, board, keep_scanning),
            Bug::Pillbug => Bug::scan_crawl_moves_while(position, board, keep_scanning),
            Bug::Queen => Bug::scan_crawl_moves_while(position, board, keep_scanning),
            Bug::Spider => Bug::scan_spider_moves_while(position, board, keep_scanning),
        }
    }

    pub fn available_abilities(
        position: Position,
        board: &Board,
    ) -> HashMap<Position, Vec<Position>> {
        if Bug::has_pillbug_throw(position, board) {
            return Bug::pillbug_throw(position, board);
        }
        HashMap::default()
    }

    pub(crate) fn has_available_ability(position: Position, board: &Board) -> bool {
        if !Bug::has_pillbug_throw(position, board) {
            return false;
        }
        if Bug::pillbug_throw_targets(position, board).next().is_none() {
            return false;
        }
        Bug::pillbug_throw_sources(position, board).any(|pos| {
            let Some(piece) = board.top_piece(pos) else {
                return false;
            };
            board.last_moved != Some((piece, pos))
        })
    }

    pub fn can_throw(
        ability_position: Position,
        thrown_position: Position,
        target_position: Position,
        board: &Board,
    ) -> bool {
        if !Bug::has_pillbug_throw(ability_position, board) {
            return false;
        }
        Bug::can_throw_piece_to(ability_position, thrown_position, target_position, board)
    }

    pub(crate) fn can_throw_piece_to(
        ability_position: Position,
        piece_position: Position,
        destination: Position,
        board: &Board,
    ) -> bool {
        if !Bug::is_canonical_position(piece_position) || !Bug::is_canonical_position(destination) {
            return false;
        }

        let destination_is_available = ability_position.is_neighbor(destination)
            && !board.occupied(destination)
            && !board.gated(2, ability_position, destination);
        if !destination_is_available {
            return false;
        }
        if !ability_position.is_neighbor(piece_position) || board.level(piece_position) > 1 {
            return false;
        }
        let Some(piece) = board.top_piece(piece_position) else {
            return false;
        };
        !board.is_pinned(piece) && !board.gated(2, piece_position, ability_position)
    }

    fn is_canonical_position(position: Position) -> bool {
        (0..BOARD_SIZE).contains(&position.q) && (0..BOARD_SIZE).contains(&position.r)
    }

    fn has_pillbug_throw(position: Position, board: &Board) -> bool {
        match board.top_bug(position) {
            Some(Bug::Pillbug) => true,
            Some(Bug::Mosquito)
                if board.level(position) == 1 && board.neighbor_is_a(position, Bug::Pillbug) =>
            {
                true
            }
            _ => false,
        }
    }

    fn crawl_negative_space<'a>(
        position: Position,
        board: &'a MidMoveBoard<'a>,
    ) -> impl Iterator<Item = Position> + 'a {
        position.positions_around().filter(move |pos| {
            let (pos1, pos2) = position.common_adjacent_positions(*pos);
            (board.occupied(pos1) || board.occupied(pos2))
                && board.is_negative_space(*pos)
                && !board.gated(1, position, *pos)
        })
    }

    fn crawl(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        board.positions_taken_around(position).flat_map(move |pos| {
            let (pos1, pos2) = position.common_adjacent_positions(pos);
            [
                (!board.gated(1, position, pos1) && !board.occupied(pos1)).then_some(pos1),
                (!board.gated(1, position, pos2) && !board.occupied(pos2)).then_some(pos2),
            ]
            .into_iter()
            .flatten()
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
        let mut found_pos = Vec::with_capacity(24);
        Bug::scan_ant_moves_while(position, board, &mut |pos| {
            found_pos.push(pos);
            true
        });
        found_pos
    }

    fn scan_ant_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        let board = MidMoveBoard::new(board, position);
        let mut unexplored = None;
        for pos in Bug::crawl_negative_space(position, &board) {
            if !keep_scanning(pos) {
                return false;
            }
            unexplored
                .get_or_insert_with(|| Vec::with_capacity(24))
                .push(pos);
        }
        let Some(mut unexplored) = unexplored else {
            return true;
        };

        let mut state = TorusArray::new((false, false));
        state.set(position, (true, true));
        for pos in &unexplored {
            state.set(*pos, (true, true));
        }

        while let Some(position) = unexplored.pop() {
            for pos in Bug::crawl_negative_space(position, &board) {
                let (found, explored) = state.get(pos);
                if !explored && !found && board.is_negative_space(pos) {
                    if !keep_scanning(pos) {
                        return false;
                    }
                    state.set(pos, (true, true));
                    unexplored.push(pos);
                }
            }
        }
        true
    }

    pub fn beetle_moves(position: Position, board: &Board) -> Vec<Position> {
        let mut positions = Vec::new();
        Bug::scan_beetle_moves_while(position, board, &mut |pos| {
            positions.push(pos);
            true
        });
        positions
    }

    fn scan_beetle_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        let mut positions = Vec::new();
        for pos in Bug::climb(position, board) {
            if !Bug::scan_unique_target_while(&mut positions, pos, keep_scanning) {
                return false;
            }
        }
        if board.level(position) == 1 {
            for pos in Bug::crawl(position, board) {
                if !Bug::scan_unique_target_while(&mut positions, pos, keep_scanning) {
                    return false;
                }
            }
        } else {
            for pos in Bug::descend(position, board) {
                if !Bug::scan_unique_target_while(&mut positions, pos, keep_scanning) {
                    return false;
                }
            }
        }
        true
    }

    pub fn grasshopper_moves(position: Position, board: &Board) -> Vec<Position> {
        let mut positions = Vec::new();
        Bug::scan_grasshopper_moves_while(position, board, &mut |pos| {
            positions.push(pos);
            true
        });
        positions
    }

    fn scan_grasshopper_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        for dir in board
            .positions_taken_around(position)
            .map(|pos| position.direction(pos))
        {
            let mut cur_pos = position;
            while board.occupied(cur_pos.to(dir)) {
                cur_pos = cur_pos.to(dir);
            }
            if !keep_scanning(cur_pos.to(dir)) {
                return false;
            }
        }
        true
    }

    fn ladybug_moves(position: Position, board: &Board) -> Vec<Position> {
        let mut positions = Vec::new();
        Bug::scan_ladybug_moves_while(position, board, &mut |pos| {
            positions.push(pos);
            true
        });
        positions
    }

    fn scan_ladybug_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        let mut positions = Vec::new();
        for first_pos in Bug::climb(position, board) {
            let current_height = board.level(first_pos) + 1;
            for second_pos in board.positions_taken_around(first_pos).filter(|pos| {
                *pos != position
                    && !board.gated(current_height.max(board.level(*pos) + 1), first_pos, *pos)
            }) {
                for third_pos in board
                    .positions_available_around(second_pos)
                    .filter(|third_pos| {
                        !board.gated(board.level(second_pos) + 1, second_pos, *third_pos)
                    })
                {
                    if !Bug::scan_unique_target_while(&mut positions, third_pos, keep_scanning) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn mosquito_moves(position: Position, board: &Board) -> Vec<Position> {
        let mut positions = Vec::new();
        Bug::scan_mosquito_moves_while(position, board, &mut |pos| {
            positions.push(pos);
            true
        });
        positions
    }

    fn scan_mosquito_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        if board.level(position) == 1 {
            for pieces in board.neighbors(position) {
                let bug = pieces.top_piece().expect("Could not get last piece").bug();
                if bug != Bug::Mosquito
                    && !Bug::scan_moves_as_bug_while(bug, position, board, keep_scanning)
                {
                    return false;
                }
            }
            true
        } else {
            Bug::scan_beetle_moves_while(position, board, keep_scanning)
        }
    }

    fn pillbug_moves(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        Bug::crawl(position, board)
    }

    fn pillbug_throw_targets(
        position: Position,
        board: &Board,
    ) -> impl Iterator<Item = Position> + '_ {
        board
            .positions_available_around(position)
            .filter(move |pos| !board.gated(2, position, *pos))
    }

    fn pillbug_throw_sources(
        position: Position,
        board: &Board,
    ) -> impl Iterator<Item = Position> + '_ {
        board.positions_taken_around(position).filter_map(move |p| {
            let piece = board.top_piece(p)?;
            (!board.is_pinned(piece) && !board.gated(2, p, position) && board.level(p) <= 1)
                .then_some(p)
        })
    }

    fn pillbug_throw(position: Position, board: &Board) -> HashMap<Position, Vec<Position>> {
        let mut moves = HashMap::default();
        // get all the positions the pillbug can throw a bug to
        let to = Bug::pillbug_throw_targets(position, board).collect::<Vec<Position>>();
        // get bugs around the pillbug that aren't pinned
        for pos in Bug::pillbug_throw_sources(position, board) {
            moves.insert(pos, to.clone());
        }
        moves
    }

    fn queen_moves(position: Position, board: &Board) -> impl Iterator<Item = Position> + '_ {
        Bug::crawl(position, board)
    }

    fn scan_crawl_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        for pos in Bug::crawl(position, board) {
            if !keep_scanning(pos) {
                return false;
            }
        }
        true
    }

    fn scan_unique_target_while(
        visited_targets: &mut Vec<Position>,
        target: Position,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        if visited_targets.contains(&target) {
            return true;
        }
        if !keep_scanning(target) {
            return false;
        }
        visited_targets.push(target);
        true
    }

    fn spider_moves(position: Position, board: &Board) -> Vec<Position> {
        let mut res = Vec::new();
        Bug::scan_spider_moves_while(position, board, &mut |pos| {
            res.push(pos);
            true
        });
        res.sort();
        res.dedup();
        res
    }

    fn scan_spider_moves_while(
        position: Position,
        board: &Board,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        let board = MidMoveBoard::new(board, position);
        let mut positions = Vec::new();
        for pos1 in Bug::crawl_negative_space(position, &board) {
            for pos2 in Bug::crawl_negative_space(pos1, &board).filter(|pos| *pos != position) {
                for pos3 in Bug::crawl_negative_space(pos2, &board)
                    .filter(|pos3| *pos3 != position && *pos3 != pos1)
                {
                    if !Bug::scan_unique_target_while(&mut positions, pos3, keep_scanning) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{color::Color, piece::Piece};

    fn assert_lazy_predicates_match_collected(board: &Board) {
        let mut checked_positions = Vec::new();
        for position in board.positions.iter().flatten() {
            if checked_positions.contains(position) {
                continue;
            }
            checked_positions.push(*position);

            let collected = Bug::normal_moves(*position, board);
            assert_eq!(
                Bug::has_move(*position, board),
                !collected.is_empty(),
                "has_move disagrees with normal_moves at {position}"
            );

            for target in &collected {
                assert!(
                    Bug::has_target_move(*position, *target, board),
                    "has_target_move missed collected move from {position} to {target}"
                );
            }

            for q in 0..BOARD_SIZE {
                for r in 0..BOARD_SIZE {
                    let target = Position::new(q, r);
                    assert_eq!(
                        Bug::has_target_move(*position, target, board),
                        collected.contains(&target),
                        "has_target_move disagrees with normal_moves from {position} to {target}"
                    );
                }
            }
        }
    }

    #[test]
    fn tests_lazy_predicates_match_collected_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        assert_lazy_predicates_match_collected(&board);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Spider, Color::White, 1),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        assert_lazy_predicates_match_collected(&board);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        board.insert(
            Position::new(-2, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.remove(Position::new(1, 0));
        assert_lazy_predicates_match_collected(&board);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        board.remove(Position::new(1, 0));
        assert_lazy_predicates_match_collected(&board);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        assert_lazy_predicates_match_collected(&board);
    }

    #[test]
    fn tests_available_moves() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
            true,
        );
        let moves = Bug::available_moves(Position::new(0, 0), &board);
        assert_eq!(
            moves.get(&Position::new(0, 0)).unwrap(),
            &Bug::normal_moves(Position::new(0, 0), &board)
        );
        assert_eq!(moves.get(&Position::new(0, 0)).unwrap().len(), 2);
        let moves = Bug::available_moves(Position::new(1, 0), &board);
        assert_eq!(
            moves.get(&Position::new(1, 0)).unwrap(),
            &Bug::normal_moves(Position::new(1, 0), &board)
        );
        assert_eq!(moves.get(&Position::new(1, 0)).unwrap().len(), 6);
    }

    #[test]
    fn tests_available_abilities() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        let positions = Bug::available_abilities(Position::new(0, 0), &board);
        let targets = positions.get(&Position::new(1, 0)).unwrap();
        assert_eq!(targets.len(), 5);
        for target in targets {
            assert!(Bug::can_throw(
                Position::new(0, 0),
                Position::new(1, 0),
                *target,
                &board
            ));
        }
        assert!(!Bug::can_throw(
            Position::new(0, 0),
            Position::new(1, 0),
            Position::new(0, 0),
            &board
        ));
        let positions = Bug::available_abilities(Position::new(1, 0), &board);
        let targets = positions.get(&Position::new(0, 0)).unwrap();
        assert_eq!(targets.len(), 5);
        for target in targets {
            assert!(Bug::can_throw(
                Position::new(1, 0),
                Position::new(0, 0),
                *target,
                &board
            ));
        }
        assert!(!Bug::can_throw(
            Position::new(1, 0),
            Position::new(0, 0),
            Position::new(1, 0),
            &board
        ));
        assert!(!Bug::can_throw(
            Position::new(0, 0),
            Position::new(1, 0),
            Position::new(2, 0),
            &board
        ));

        assert!(!Bug::can_throw(
            Position::new(0, 0),
            Position::new(2, 0),
            Position::new(0, 1),
            &board
        ));
    }

    #[test]
    fn tests_can_throw_rejects_non_canonical_positions() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        assert!(!Bug::can_throw(
            Position::new(0, 0),
            Position::new(1, 0),
            Position { q: -1, r: 0 },
            &board
        ));

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        assert!(!Bug::can_throw(
            Position::new(0, 0),
            Position { q: -1, r: 0 },
            Position::new(0, 1),
            &board
        ));

        let mut board = Board::new();
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        assert!(!Bug::can_throw(
            Position::new(-1, 0),
            Position::new(0, 0),
            Position {
                q: BOARD_SIZE,
                r: 0
            },
            &board
        ));
    }

    #[test]
    fn tests_pillbug_throw() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        let positions = Bug::pillbug_throw(Position::new(0, 0), &board);
        assert_eq!(positions.get(&Position::new(1, 0)).unwrap().len(), 5);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Pillbug, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 0);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 5);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
            true,
        );
        let positions = Bug::mosquito_moves(Position::new(0, 0), &board);
        assert_eq!(positions.len(), 2);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(0, 1),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
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
            true,
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::NE),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::NE),
            Piece::new_from(Bug::Beetle, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::SE),
            Piece::new_from(Bug::Ant, Color::White, 2),
            true,
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::SE),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
            true,
        );
        let positions = Bug::climb(Position::new(1, 0), &board);
        assert_eq!(positions.count(), 1);
        let mut positions = Bug::climb(Position::new(1, 0), &board);
        assert!(positions.any(|pos| pos == Position::new(0, 0)));

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
            let positions = Bug::climb(Position::new(0, 0), &board);
            assert_eq!(positions.count(), i + 1);
        }

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::NE),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, 0).to(crate::direction::Direction::SE),
            Piece::new_from(Bug::Beetle, Color::Black, 2),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&Position::new(0, 0).to(Direction::NE)));
        assert!(positions.contains(&Position::new(0, 0).to(Direction::SE)));

        // two adjacent neighbors means two positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(0, 1),
            Piece::new_from(Bug::Ant, Color::White, 1),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(-1, 1),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
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
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::NE),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::SE),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::W),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
        );
        let positions = Bug::crawl(Position::new(0, 0), &board).collect::<Vec<_>>();
        assert_eq!(positions.len(), 0);

        // three neighbors no gate -> 2 positions
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::White, 1),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::NE),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::E),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::SE),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
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
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::NE),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::E),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::SE),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
        );
        board.insert(
            Position::new(0, 0).to(Direction::SW),
            Piece::new_from(Bug::Ladybug, Color::Black, 1),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(0, 1),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
        );
        board.insert(
            Position::new(-1, 1),
            Piece::new_from(Bug::Ladybug, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
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
            true,
        );
        board.insert(
            Position::new(-1, 0),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(-2, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        assert_eq!(Bug::ladybug_moves(Position::new(0, 0), &board).len(), 5);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        board.remove(Position::new(1, 0));
        assert_eq!(Bug::ladybug_moves(Position::new(0, 0), &board).len(), 12);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Ladybug, Color::White, 0),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        board.insert(
            Position::new(-2, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        assert_eq!(Bug::beetle_moves(Position::new(0, 0), &board).len(), 4);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        assert_eq!(Bug::beetle_moves(Position::new(0, 0), &board).len(), 6);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
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
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::White, 1),
            true,
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
            true,
        );
        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            board.insert(
                pos,
                Piece::new_from(Bug::Ant, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        assert_eq!(Bug::grasshopper_moves(Position::new(0, 0), &board).len(), 6);

        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Grasshopper, Color::White, 1),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Beetle, Color::Black, 1),
            true,
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
            true,
        );
        assert_eq!(Bug::grasshopper_moves(Position::new(0, 0), &board).len(), 0);
    }
}
