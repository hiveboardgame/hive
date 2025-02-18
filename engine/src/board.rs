use crate::direction::Direction;
use crate::hasher::Hasher;
use crate::position::{CircleIter, Rotation};
use crate::{
    bug::Bug, bug_stack::BugStack, color::Color, dfs_info::DfsInfo, game_error::GameError,
    game_result::GameResult, game_type::GameType, piece::Piece, position::Position,
    torus_array::TorusArray,
};
use anyhow::Result;
use itertools::Itertools;
use lazy_static::lazy_static;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::fs::OpenOptions;
use std::path::PathBuf;

pub const BOARD_SIZE: i32 = 32;
lazy_static! {
    static ref BLACK_QUEEN: Piece = Piece::new_from(Bug::Queen, Color::Black, 0);
    static ref WHITE_QUEEN: Piece = Piece::new_from(Bug::Queen, Color::White, 0);
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board {
    pub board: TorusArray<BugStack>,
    pub neighbor_count: TorusArray<u8>,
    // last moved contains the piece that was last moved
    pub last_moved: Option<(Piece, Position)>,
    // last move contains a from and to position of the last move
    pub last_move: (Option<Position>, Option<Position>),
    pub stunned: Option<Piece>,
    pub positions: [Option<Position>; 48],
    //   wA1, wA2, wA3, wB1, wB2, ...
    // [ qr   N     qr  N     N
    // offset_to_piece
    // for every piece: bottom?
    pinned: [bool; 48],
    pub played: usize,
    pub hasher: Hasher,
    pub smallest: Option<(Piece, Position)>,
    pub eigen_direction: Option<Direction>,
}

impl Board {
    pub fn new() -> Self {
        Self {
            board: TorusArray::new(BugStack::new()),
            neighbor_count: TorusArray::new(0),
            // TODO: @leex implement a cache for which pieces currently control the main direction
            // invalidate when a lower piece gets played invalidate when one of the pieces moves
            // circle_indexes: TorusArray::new(0),
            stunned: None,
            last_moved: None,
            last_move: (None, None),
            positions: [None; 48],
            pinned: [false; 48],
            played: 0,
            hasher: Hasher::new(),
            smallest: None,
            eigen_direction: None,
        }
    }

    pub fn create_svg(&self, mut path: PathBuf) -> Result<()> {
        path.set_extension("svg");
        let file = OpenOptions::new()
            .read(true)
            .write(true) // Required for creation
            .create(true)
            .truncate(true)
            .open(path)?;
        for (offset, position) in self.positions.iter().enumerate() {
            let piece = Self::offset_to_piece(offset);
            if let Some(pos) = position {
                let level = self
                    .level_of_piece(piece, *pos)
                    .expect("TODO get rid of this expect");
                println!("Position: {} Piece: {} Level: {}", pos, piece, level);
            }
        }
        Ok(())
    }

    // this always gets called as a last step
    pub fn hash_move(
        &mut self,
        piece: Piece,
        from: Option<Position>,
        to: Position,
        turn: usize,
    ) -> u64 {
        if self.played == 1 {
            let bs = self.board.get_mut(Position::initial_spawn_position());
            bs.index = [Some(0), Some(0)];
            self.smallest = Some((piece, to));
            self.hasher.update(bs, Some(0), Rotation::C);
            self.hasher.update(bs, Some(0), Rotation::CC);
            self.hasher.finish_turn(self.stunned);
            return self.hasher.hash;
        }
        let mut smallest_unchanged = true;
        let smallest = self.get_smallest(piece, to);
        if self.smallest != smallest {
            self.smallest = smallest;
            smallest_unchanged = false;
        }
        let eigen_direction = self.eigen_direction();
        if self.eigen_direction == Some(eigen_direction) && smallest_unchanged {
            // IF THIS IS A MOVE SO CLEAR THE FROM
            if let Some(from) = from {
                let mut stack = self.board.get(from).clone();
                self.hasher.update(&stack, None, Rotation::C);
                self.hasher.update(&stack, None, Rotation::CC);
                stack.push_piece(piece);
                self.hasher.update(&stack, None, Rotation::C);
                self.hasher.update(&stack, None, Rotation::CC);
            }

            let mut stack = self.board.get(to).clone();
            if stack.index != [None, None] {
                let top_piece = stack.pop_piece();
                debug_assert_eq!(piece, top_piece);
                self.hasher.update(&stack, None, Rotation::C);
                self.hasher.update(&stack, None, Rotation::CC);
            }

            let smallest_position = self.smallest.unwrap().1;
            let clockwise = CircleIter::new(smallest_position, eigen_direction, Rotation::C);
            let counter_clockwise =
                CircleIter::new(smallest_position, eigen_direction, Rotation::CC);
            let (c_index, cc_index) = (
                clockwise.take_while(|pos| *pos != to).count(),
                counter_clockwise.take_while(|pos| *pos != to).count(),
            );
            let stack = self.board.get_mut(to);
            stack.index[Rotation::C as usize] = Some(c_index);
            stack.index[Rotation::CC as usize] = Some(cc_index);
            self.hasher.update(stack, Some(c_index as u32), Rotation::C);
            self.hasher
                .update(stack, Some(cc_index as u32), Rotation::CC);
        } else {
            self.eigen_direction = Some(eigen_direction);
            self.hasher.clear(turn);
            let smallest_position = self.smallest.unwrap().1;
            let clockwise = CircleIter::new(smallest_position, eigen_direction, Rotation::C);
            let mut hashed = 0_usize;
            for (index, position) in clockwise.enumerate() {
                let bs = self.board.get_mut(position);
                bs.index[Rotation::C as usize] = Some(index);
                if !bs.is_empty() {
                    self.hasher.update(bs, Some(index as u32), Rotation::C);
                    hashed += bs.size as usize;
                }
                if hashed == self.played {
                    break;
                }
            }
            let counter_clockwise =
                CircleIter::new(smallest_position, eigen_direction, Rotation::CC);
            hashed = 0_usize;
            for (index, position) in counter_clockwise.enumerate() {
                let bs = self.board.get_mut(position);
                bs.index[Rotation::CC as usize] = Some(index);
                if !bs.is_empty() {
                    self.hasher.update(bs, Some(index as u32), Rotation::CC);
                    hashed += bs.size as usize;
                }
                if hashed == self.played {
                    break;
                }
            }
        }
        self.hasher.finish_turn(self.stunned)
    }

    pub fn ring_is_empty(&self, mut position: Position, ring: usize) -> bool {
        for _ in 0..ring {
            position = position.to(Direction::W);
        }
        let mut direction = Direction::W.next_direction(Rotation::C);
        for _ in 0..6 {
            direction = direction.next_direction(Rotation::C);
            for _ in 0..ring {
                if !self.board.get(position).is_empty() {
                    return false;
                };
                position = position.to(direction);
            }
        }
        true
    }

    pub fn walk_ring(
        &self,
        mut direction: Direction,
        mut position: Position,
        ring: usize,
        revolution: Rotation,
    ) -> String {
        let mut found = String::new();
        for _ in 0..ring {
            position = position.to(direction);
        }
        direction = direction.next_direction(revolution);
        for _ in 0..6 {
            direction = direction.next_direction(revolution);
            for _ in 0..ring {
                found.push(self.board.get(position).to_char());
                position = position.to(direction);
            }
        }
        found
    }

    pub fn eigen_direction(&self) -> Direction {
        let (_, smallest_position) = self.smallest.unwrap();
        let mut candidates = Vec::new();
        for position in self.positions_taken_around(smallest_position) {
            let candidate = self.board.get(position).smallest().unwrap();
            if candidates.is_empty() {
                candidates.push((candidate, position));
            } else {
                match candidate
                    .simple()
                    .cmp(&candidates.first().unwrap().0.simple())
                {
                    Ordering::Less => candidates = vec![(candidate, position)],
                    Ordering::Equal => candidates.push((candidate, position)),
                    Ordering::Greater => {}
                }
            }
        }
        if candidates.len() > 1 {
            return self.eigen_direction_tie_breaker(2);
        }
        smallest_position.direction(candidates.first().unwrap().1)
    }

    pub fn eigen_direction_tie_breaker(&self, ring: usize) -> Direction {
        let (_smallest_piece, smallest_position) = self.smallest.unwrap();
        let mut h: HashMap<String, Vec<Direction>> = HashMap::new();
        let mut backup: HashMap<String, Vec<Direction>> = HashMap::new();
        for dir in Direction::all().iter() {
            let c = self.walk_ring(*dir, smallest_position, ring, Rotation::C);
            let cc = self.walk_ring(*dir, smallest_position, ring, Rotation::CC);
            if c < cc {
                h.entry(c).or_default().push(*dir);
            } else {
                h.entry(cc).or_default().push(*dir);
            }
        }
        if self.ring_is_empty(smallest_position, ring + 1) {
            backup.clone_from(&h);
        }
        h.retain(|_, dirs| dirs.len() == 1);
        if !h.is_empty() {
            *h[h.keys().min().unwrap()].first().unwrap()
        } else if backup.is_empty() {
            self.eigen_direction_tie_breaker(ring + 1)
        } else {
            *backup[backup.keys().min().unwrap()].first().unwrap()
        }
    }

    pub fn find_sextant(&self, from: Position, to: Piece) -> Direction {
        for level in 1..27 {
            for direction in Direction::all().into_iter() {
                if self
                    .explore_sextant_level(to, from, direction, level)
                    .is_some()
                {
                    return direction;
                }
            }
        }
        panic!(
            "{} Finding From position: {} to piece: {} failed.",
            self, from, to
        );
    }

    pub fn explore_sextant_level(
        &self,
        find: Piece,
        mut position: Position,
        mut direction: Direction,
        level: usize,
    ) -> Option<Position> {
        for _ in 0..level {
            position = position.to(direction);
        }
        direction = direction.next_direction_120();
        for _ in 0..level {
            // TODO: @leex swap .top_piece for bug_stack.contains(piece)
            if self.board.get(position).contains(&find) {
                return Some(position);
            }
            position = position.to(direction);
        }
        None
    }

    pub fn is_shutout(&self, color: Color, game_type: GameType) -> bool {
        if let GameResult::Unknown = self.game_result() {
            return self.moves(color).is_empty()
                && (self.spawnable_positions(color).next().is_none()
                    || self.reserve(color, game_type).is_empty());
        };
        false
    }

    pub fn game_result(&self) -> GameResult {
        let black_won = self
            .position_of_piece(*WHITE_QUEEN)
            .map(|pos| self.neighbors(pos).count() == 6);
        let white_won = self
            .position_of_piece(*BLACK_QUEEN)
            .map(|pos| self.neighbors(pos).count() == 6);
        match (black_won, white_won) {
            (Some(true), Some(true)) => GameResult::Draw,
            (Some(true), Some(false)) => GameResult::Winner(Color::Black),
            (Some(false), Some(true)) => GameResult::Winner(Color::White),
            _ => GameResult::Unknown,
        }
    }

    pub fn set_position_of_piece(&mut self, piece: Piece, position: Position) {
        self.positions[self.piece_to_offset(piece)] = Some(position);
    }

    pub fn position_of_piece(&self, piece: Piece) -> Option<Position> {
        *self
            .positions
            .get(self.piece_to_offset(piece))
            .expect("The vec gets initialized to have space for all the bugs")
    }

    pub fn piece_already_played(&self, piece: Piece) -> bool {
        self.position_of_piece(piece).is_some()
    }

    pub fn move_piece(
        &mut self,
        piece: Piece,
        current: Position,
        target: Position,
        turn: usize,
    ) -> Result<(), GameError> {
        if !self.is_top_piece(piece, current) {
            return Err(GameError::InvalidMove {
                piece: piece.to_string(),
                from: current.to_string(),
                to: target.to_string(),
                turn,
                reason: "Trying to move a covered piece".to_string(),
            });
        }

        let removed_piece = self.remove(current);
        debug_assert_eq!(removed_piece, piece);
        self.insert(target, piece, false);
        Ok(())
    }

    pub fn remove(&mut self, position: Position) -> Piece {
        let bug_stack = self.board.get_mut(position);
        let piece = bug_stack.pop_piece();
        if bug_stack.is_empty() {
            self.neighbor_count_remove(position);
        }
        piece
    }

    pub fn check(&self) -> bool {
        // This function can be used to perform checks on the engine and for debugging engine
        // issues on every turn
        true
        // for this remove the return true and then implement your check in the loop
        // for r in 0..32 {
        //     for q in 0..32 {
        //         let position = Position::new(q, r);
        //         let hex = self.board.get(position);
        //         let neighbor_count = *self.neighbor_count.get(position);
        //         let counted = self.positions_taken_around(position).count();
        //         if counted != neighbor_count as usize {
        //             println!("Calculated: {counted} hashed: {neighbor_count}");
        //             println!("pos: {position}");
        //             println!("hex: {hex:?}");
        //             println!("{}", self);
        //             return false;
        //         }
        //     }
        // }
    }

    pub fn slow_test_negative_space(&self, position: Position) -> bool {
        !self.occupied(position) && self.has_neighbor(position)
    }

    pub fn neighbor_count_remove(&mut self, position: Position) {
        for pos in position.positions_around() {
            *self.neighbor_count.get_mut(pos) -= 1;
        }
    }

    pub fn neighbor_count_add(&mut self, position: Position) {
        for pos in position.positions_around() {
            *self.neighbor_count.get_mut(pos) += 1;
        }
    }

    pub fn neighbor_is_a(&self, position: Position, bug: Bug) -> bool {
        self.top_layer_neighbors(position)
            .any(|piece| piece.bug() == bug)
    }

    pub fn level(&self, position: Position) -> usize {
        self.board.get(position).size as usize
    }

    pub fn piece_to_offset(&self, piece: Piece) -> usize {
        piece.color() as usize * 24 + piece.bug() as usize * 3 + piece.order().saturating_sub(1)
    }

    pub fn offset_to_piece(offset: usize) -> Piece {
        let color = offset as u8 / 24;
        let bug = (offset as u8 - color * 24) / 3;
        let order = (offset as u8 + 1 - bug * 3 - color * 24) as usize;
        Piece::new_from(Bug::from(bug), Color::from(color), order)
    }

    pub fn is_pinned(&self, piece: Piece) -> bool {
        let position = self
            .position_of_piece(piece)
            .expect("Piece not found on board");
        self.pinned[self.piece_to_offset(piece)] && self.board.get(position).size == 1
    }

    pub fn bottom_piece(&self, position: Position) -> Option<Piece> {
        self.board.get(position).bottom_piece()
    }

    pub fn top_piece(&self, position: Position) -> Option<Piece> {
        self.board.get(position).top_piece()
    }

    pub fn under_piece(&self, position: Position) -> Option<Piece> {
        self.board.get(position).under_piece()
    }

    pub fn level_of_piece(&self, piece: Piece, position: Position) -> Option<usize> {
        self.board
            .get(position)
            .pieces
            .iter()
            .position(|e| *e == piece)
    }

    pub fn is_bottom_piece(&self, piece: Piece, position: Position) -> bool {
        self.bottom_piece(position)
            .map(|found| found == piece)
            .unwrap_or(false)
    }

    pub fn is_top_piece(&self, piece: Piece, position: Position) -> bool {
        self.top_piece(position)
            .map(|found| found == piece)
            .unwrap_or(false)
    }

    pub fn top_bug(&self, position: Position) -> Option<Bug> {
        if let Some(piece) = self.top_piece(position) {
            return Some(piece.bug());
        }
        None
    }

    pub fn gated(&self, level: usize, from: Position, to: Position) -> bool {
        let (pos1, pos2) = from.common_adjacent_positions(to);
        let p1 = self.board.get(pos1);
        let p2 = self.board.get(pos2);
        if p1.is_empty() || p2.is_empty() {
            return false;
        }
        p1.len() >= level && p2.len() >= level
    }

    pub fn get_neighbor(&self, position: Position) -> Option<(Piece, Position)> {
        for pos in position.positions_around() {
            if let Some(piece) = self.top_piece(pos) {
                return Some((piece, pos));
            }
        }
        None
    }

    fn has_neighbor(&self, position: Position) -> bool {
        for pos in position.positions_around() {
            if self.occupied(pos) {
                return true;
            }
        }
        false
    }

    pub fn positions_taken_around(
        &self,
        position: Position,
    ) -> impl Iterator<Item = Position> + '_ {
        position
            .positions_around()
            .filter(|pos| self.occupied(*pos))
    }

    pub fn occupied(&self, position: Position) -> bool {
        self.board.get(position).size > 0
    }

    pub fn positions_available_around(
        &self,
        position: Position,
    ) -> impl Iterator<Item = Position> + '_ {
        position
            .positions_around()
            .filter(|pos| !self.occupied(*pos))
    }

    pub fn neighbors(&self, position: Position) -> impl Iterator<Item = BugStack> + '_ {
        position.positions_around().filter_map(move |pos| {
            if self.occupied(pos) {
                Some(self.board.get(pos).clone())
            } else {
                None
            }
        })
    }

    pub fn is_valid_move(
        &self,
        color: Color,
        piece: Piece,
        current_position: Position,
        target_position: Position,
    ) -> bool {
        match self.moves(color).get(&(piece, current_position)) {
            None => false,
            Some(positions) => positions.contains(&target_position),
        }
    }

    pub fn moves(&self, color: Color) -> HashMap<(Piece, Position), Vec<Position>> {
        let mut moves: HashMap<(Piece, Position), Vec<Position>> = HashMap::default();
        match self.game_result() {
            GameResult::Unknown => {}
            _ => return moves,
        }
        if !self.queen_played(color) {
            return moves;
        }
        for pos in self.positions.iter().flatten() {
            if let Some(piece) = self.top_piece(*pos) {
                if piece.is_color(color) {
                    // let's make sure pieces that were just moved cannot be moved again
                    if let Some(last_moved) = self.last_moved {
                        if last_moved == (piece, *pos) {
                            // now we skip it
                            continue;
                        }
                    }
                    for (start_pos, target_positions) in Bug::available_moves(*pos, self) {
                        if let Some(piece) = self.top_piece(start_pos) {
                            if !target_positions.is_empty() {
                                moves
                                    .entry((piece, start_pos))
                                    .or_default()
                                    .append(&mut target_positions.clone());
                            }
                        }
                    }
                }
            }
        }

        if let Some(last_moved) = self.last_moved {
            moves.remove(&last_moved);
        }
        moves
    }

    pub fn spawnable_positions(&self, color: Color) -> impl Iterator<Item = Position> + '_ {
        std::iter::once(Position::initial_spawn_position())
            .chain(self.negative_space())
            .filter(move |pos| self.spawnable(color, *pos))
    }

    pub fn queen_played(&self, color: Color) -> bool {
        self.piece_already_played(Piece::new_from(Bug::Queen, color, 0))
    }

    pub fn queen_required(&self, turn: usize, color: Color) -> bool {
        if turn == 6 && color == Color::White && !self.queen_played(Color::White) {
            return true;
        }
        if turn == 7 && color == Color::Black && !self.queen_played(Color::Black) {
            return true;
        }
        false
    }

    pub fn update_pinned(&mut self) {
        for pinned_info in self.calculate_pinned().iter() {
            self.pinned[self.piece_to_offset(pinned_info.piece)] = pinned_info.pinned
        }
    }

    pub fn calculate_pinned(&self) -> Vec<DfsInfo> {
        // make sure to get only top pieces in this
        let mut dfs_info = self
            .positions
            .iter()
            .enumerate()
            .filter_map(|(i, maybe_pos)| {
                if let Some(pos) = maybe_pos {
                    if self.is_bottom_piece(Self::offset_to_piece(i), *pos) {
                        Some(DfsInfo {
                            position: *pos,
                            piece: self.bottom_piece(*pos).unwrap(),
                            visited: false,
                            depth: 0,
                            low: 0,
                            pinned: false,
                            parent: None,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if dfs_info.is_empty() {
            return dfs_info;
        }
        self.bcc(0, 0, &mut dfs_info);
        dfs_info
    }

    pub fn bcc(&self, i: usize, d: usize, dfs_info: &mut Vec<DfsInfo>) {
        dfs_info[i].visited = true;
        dfs_info[i].depth = d;
        dfs_info[i].low = d;
        let mut child_count = 0;
        let mut ap = false;

        for pos in self.positions_taken_around(dfs_info[i].position) {
            let ni = dfs_info.iter().position(|e| e.position == pos).unwrap();
            if !dfs_info[ni].visited {
                child_count += 1;
                dfs_info[ni].parent = Some(i);
                self.bcc(ni, d + 1, dfs_info);
                if dfs_info[ni].low >= dfs_info[i].depth {
                    ap = true;
                }
                dfs_info[i].low = std::cmp::min(dfs_info[i].low, dfs_info[ni].low);
            } else if dfs_info[i].parent.is_some() && ni != dfs_info[i].parent.unwrap() {
                dfs_info[i].low = std::cmp::min(dfs_info[i].low, dfs_info[ni].depth);
            }
        }
        if dfs_info[i].parent.is_some() && ap || (dfs_info[i].parent.is_none() && child_count > 1) {
            dfs_info[i].pinned = true;
        }
    }

    pub fn top_layer_neighbors(&self, position: Position) -> impl Iterator<Item = Piece> + '_ {
        position
            .positions_around()
            .filter_map(|pos| self.board.get(pos).top_piece())
    }

    pub fn reserve(&self, color: Color, game_type: GameType) -> HashMap<Bug, Vec<String>> {
        let mut res = HashMap::<Bug, Vec<String>>::new();
        let start = 24 * color as usize;
        let end = 24 + start;
        let bugs_for_game_type = Bug::bugs_count(game_type);
        for (i, maybe_pos) in self.positions[start..end].iter().enumerate() {
            if maybe_pos.is_none() {
                let piece = Self::offset_to_piece(i + 24 * color as usize);
                if let Some(number_of_bugs) = bugs_for_game_type.get(&piece.bug()) {
                    if (*number_of_bugs as usize) > (i % 3) {
                        res.entry(piece.bug()).or_default().push(piece.to_string());
                    }
                }
            }
        }
        res
    }

    pub fn all_taken_positions(&self) -> impl Iterator<Item = Position> {
        // TODO this does not uniq!
        self.positions.into_iter().flatten()
    }

    pub fn center_coordinates(&self) -> Position {
        let positions = self.all_taken_positions().collect::<Vec<_>>();
        //center won't shift much if any in the first few moves
        if positions.len() < 8 {
            return Position::initial_spawn_position();
        }

        let (q_min, q_max, r_min, r_max) = positions.iter().fold(
            (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
            |(q_min, q_max, r_min, r_max), pos| {
                (
                    q_min.min(pos.q),
                    q_max.max(pos.q),
                    r_min.min(pos.r),
                    r_max.max(pos.r),
                )
            },
        );
        //TODO: Some look centered with q + 1 some without it, figure out something
        Position {
            q: q_min + ((q_max - q_min) / 2),
            r: r_min + ((r_max - r_min) / 2),
        }
    }

    pub fn spawnable(&self, color: Color, position: Position) -> bool {
        match self.game_result() {
            GameResult::Unknown => {}
            _ => return false,
        }
        if self.occupied(position) {
            return false;
        }
        // TODO maybe hand in state.turn and get rid of this
        let number_of_positions = self.positions.into_iter().flatten().count();
        if number_of_positions == 0 {
            return position == Position::initial_spawn_position();
        }
        if number_of_positions == 1 {
            return self.is_negative_space(position);
        }
        // connected to the hive
        if self.top_layer_neighbors(position).next().is_none() {
            return false;
        }
        !self
            .top_layer_neighbors(position)
            .any(|piece| color == piece.color().opposite_color())
    }

    pub fn negative_space(&self) -> impl Iterator<Item = Position> + '_ {
        Self::all_positions().filter(move |pos| self.is_negative_space(*pos))
    }

    pub fn is_negative_space(&self, position: Position) -> bool {
        !self.occupied(position) && *self.neighbor_count.get(position) > 0
    }

    pub fn get_smallest(&mut self, piece: Piece, position: Position) -> Option<(Piece, Position)> {
        if matches!(self.smallest, Some((piece, _)) if piece == *WHITE_QUEEN) {
            return Some((*WHITE_QUEEN, self.position_of_piece(*WHITE_QUEEN).unwrap()));
        }
        if let Some((current_piece, current_position)) = self.smallest {
            return match piece.simple().cmp(&current_piece.simple()) {
                Ordering::Less => Some((piece, position)),
                Ordering::Greater => self.smallest,
                Ordering::Equal => {
                    // To make this unique in the case an equal piece gets played, we go with the
                    // one with the bigger neighbors
                    if self
                        .neighbors(current_position)
                        .map(|bs| bs.simple())
                        .reduce(|acc, e| acc + e)
                        < self
                            .neighbors(position)
                            .map(|bs| bs.simple())
                            .reduce(|acc, e| acc + e)
                    {
                        Some((piece, position))
                    } else {
                        self.smallest
                    }
                }
            };
        }
        Some((piece, position))
    }

    pub fn set_stunned(&mut self, position: Position, piece: Piece, spawn: bool) {
        if spawn {
            self.stunned = None;
            return;
        }
        let mut stunned = None;
        for n in self.top_layer_neighbors(position) {
            if n.color() != piece.color() && n.bug() == Bug::Pillbug
                || n.bug() == Bug::Mosquito
                    && self
                        .top_layer_neighbors(
                            self.position_of_piece(n).expect("Piece to have a position"),
                        )
                        .any(|nn| nn.bug() == Bug::Pillbug)
            {
                stunned = Some(piece);
            }
        }
        self.stunned = stunned;
    }

    pub fn insert(&mut self, position: Position, piece: Piece, spawn: bool) {
        self.last_moved = Some((piece, position));
        let stack = self.board.get_mut(position);
        stack.push_piece(piece);
        self.set_position_of_piece(piece, position);
        if self.board.get(position).size == 1 {
            self.neighbor_count_add(position)
        }
        self.update_pinned();
        if spawn {
            self.played += 1;
        }
        self.set_stunned(position, piece, spawn);
    }

    pub fn all_positions() -> impl Iterator<Item = Position> {
        (0..BOARD_SIZE)
            .cartesian_product(0..BOARD_SIZE)
            .map(|(q, r)| Position { q, r })
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = "".to_string();
        for r in 0..BOARD_SIZE {
            if r.rem_euclid(2) == 1 {
                write!(s, "  ")?;
            }
            for q in 0..BOARD_SIZE {
                let bug_stack = self.board.get(Position::new(q - r / 2, r + 15));
                if let Some(last) = bug_stack.top_piece() {
                    if last.to_string().len() < 3 {
                        write!(s, "{last}  ")?;
                    } else {
                        write!(s, "{last} ")?;
                    }
                } else {
                    write!(s, "    ")?;
                }
            }
            writeln!(s)?;
        }
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tests_positions_around() {
        let positions_0_0 = Position::new(0, 0)
            .positions_around()
            .collect::<HashSet<Position>>();
        for pos in positions_0_0.clone().into_iter() {
            let other = pos.positions_around().collect::<HashSet<Position>>();
            assert_eq!(positions_0_0.intersection(&other).count(), 2);
        }
    }

    #[test]
    fn tests_positions_taken_around_iter() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        let pos = board
            .positions_taken_around(Position::new(0, 0))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![Position::new(1, 0)]);
    }

    #[test]
    fn tests_neighbors() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.check();
        let mut bug_stack = BugStack::new();
        let piece = Piece::new_from(Bug::Ant, Color::Black, 1);
        bug_stack.push_piece(piece);
        board.insert(
            Position::new(1, 0),
            bug_stack.top_piece().expect("This is in test neighbors"),
            true,
        );
        let neighbors = board
            .neighbors(Position::new(0, 0))
            .map(|b| b.pieces)
            .collect::<Vec<_>>();
        assert_eq!(neighbors, vec![bug_stack.pieces]);

        bug_stack.push_piece(Piece::new_from(Bug::Beetle, Color::Black, 1));
        board.insert(
            Position::new(1, 0),
            bug_stack.top_piece().expect("This is in test neighbors"),
            true,
        );
        let neighbors = board
            .neighbors(Position::new(0, 0))
            .map(|b| b.pieces)
            .collect::<Vec<_>>();
        assert_eq!(neighbors, vec![bug_stack.pieces]);

        board.insert(
            Position::new(0, 2),
            Piece::new_from(Bug::Ladybug, Color::Black, 0),
            true,
        );
        let neighbors = board
            .neighbors(Position::new(0, 0))
            .map(|b| b.pieces)
            .collect::<Vec<_>>();
        assert_eq!(neighbors, vec![bug_stack.pieces]);
    }

    #[test]
    fn tests_top_layer_neighbors() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(2, 0),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(3, 0),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
        );
        board.insert(
            Position::new(4, 0),
            Piece::new_from(Bug::Grasshopper, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(3, 1),
            Piece::new_from(Bug::Grasshopper, Color::Black, 2),
            true,
        );
        assert_eq!(
            board
                .top_layer_neighbors(Position::new(0, 0))
                .collect::<Vec<_>>()
                .len(),
            1
        );
        assert_eq!(
            board
                .top_layer_neighbors(Position::new(1, 0))
                .collect::<Vec<_>>()
                .len(),
            2
        );
        assert_eq!(
            board
                .top_layer_neighbors(Position::new(2, 0))
                .collect::<Vec<_>>()
                .len(),
            2
        );
        assert_eq!(
            board
                .top_layer_neighbors(Position::new(3, 0))
                .collect::<Vec<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn tests_negative_space() {
        let mut board = Board::new();
        board.insert(
            Position::initial_spawn_position(),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        for pos in Position::initial_spawn_position().positions_around() {
            assert!(board.is_negative_space(pos));
        }
        board.insert(
            Position::initial_spawn_position().to(Direction::NW),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        assert_eq!(board.negative_space().count(), 8);
    }

    #[test]
    fn tests_spawnable_positions() {
        let mut board = Board::new();
        board.insert(
            Position::initial_spawn_position(),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        board.insert(
            Position::initial_spawn_position().to(Direction::E),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        let positions = board.spawnable_positions(Color::Black);
        assert_eq!(positions.count(), 3);
        let positions = board.spawnable_positions(Color::White);
        assert_eq!(positions.count(), 3);
        board.insert(
            Position::initial_spawn_position()
                .to(Direction::E)
                .to(Direction::E),
            Piece::new_from(Bug::Ant, Color::White, 2),
            true,
        );
        let positions = board.spawnable_positions(Color::White);
        assert_eq!(positions.count(), 6);
        let positions = board.spawnable_positions(Color::Black);
        assert_eq!(positions.count(), 0);
    }

    #[test]
    fn tests_spawnable() {
        let mut board = Board::new();
        // if board is empty you can spawn
        assert!(board.spawnable(Color::White, Position::initial_spawn_position()));
        board.insert(
            Position::initial_spawn_position(),
            Piece::new_from(Bug::Ant, Color::White, 1),
            true,
        );

        // if position is already occupied, a bug can't be spawned there
        assert!(!board.spawnable(Color::White, Position::initial_spawn_position()));

        // the second bug can always be played
        assert!(board.spawnable(
            Color::Black,
            Position::initial_spawn_position().to(Direction::E)
        ));
        board.insert(
            Position::initial_spawn_position().to(Direction::E),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );

        // now no other black bug can be spawned around the white one
        for pos in Position::initial_spawn_position().positions_around() {
            assert!(!board.spawnable(Color::Black, pos));
        }

        // a white bug can be added adjacent to a white, but not a black bug
        assert!(!board.spawnable(
            Color::White,
            Position::initial_spawn_position()
                .to(Direction::E)
                .to(Direction::E)
        ));
        assert!(board.spawnable(
            Color::White,
            Position::initial_spawn_position().to(Direction::W)
        ));
        assert!(board.spawnable(
            Color::Black,
            Position::initial_spawn_position()
                .to(Direction::E)
                .to(Direction::E)
        ));
        assert!(!board.spawnable(
            Color::Black,
            Position::initial_spawn_position().to(Direction::W)
        ));
    }

    #[test]
    fn tests_move_splits_hive() {
        let mut board = Board::new();
        board.insert(
            Position::new(0, 0),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            Position::new(1, 0),
            Piece::new_from(Bug::Ant, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(2, 0),
            Piece::new_from(Bug::Ant, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(3, 0),
            Piece::new_from(Bug::Ant, Color::Black, 3),
            true,
        );
        assert!(!board.is_pinned(Piece::new_from(Bug::Queen, Color::Black, 1)));
        println!("{board}");
        assert!(board.is_pinned(Piece::new_from(Bug::Ant, Color::Black, 1)));
        assert!(board.is_pinned(Piece::new_from(Bug::Ant, Color::Black, 2)));
        assert!(!board.is_pinned(Piece::new_from(Bug::Ant, Color::Black, 3)));

        for (i, pos) in Position::new(0, 0).positions_around().enumerate() {
            if pos == Position::new(1, 0) {
                continue;
            }
            println!("{board}");
            println!(
                "pos: {pos}, piece: {}",
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1)
            );
            board.insert(
                pos,
                Piece::new_from(Bug::Grasshopper, Color::from((i % 2) as u8), i / 2 + 1),
                true,
            );
        }
        for pos in Position::new(0, 0).positions_around() {
            if pos == Position::new(1, 0) {
                assert!(board.is_pinned(board.top_piece(pos).unwrap()));
            } else {
                assert!(!board.is_pinned(board.top_piece(pos).unwrap()));
            };
        }
    }

    #[test]
    pub fn tests_positions_taken_around() {
        let mut board = Board::new();
        let pos = Position::new(0, 0);
        board.insert(pos, Piece::new_from(Bug::Queen, Color::Black, 0), true);
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
            Position::new(0, 1),
            Piece::new_from(Bug::Spider, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(0, -1),
            Piece::new_from(Bug::Spider, Color::Black, 2),
            true,
        );
        board.insert(
            Position::new(1, -1),
            Piece::new_from(Bug::Grasshopper, Color::Black, 1),
            true,
        );
        board.insert(
            Position::new(-1, 1),
            Piece::new_from(Bug::Grasshopper, Color::Black, 2),
            true,
        );
        assert_eq!(board.positions_taken_around(pos).count(), 6);
        for pos in pos.positions_around() {
            assert_eq!(board.positions_taken_around(pos).count(), 3);
        }
    }
}
