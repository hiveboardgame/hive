use crate::{
    bug::Bug,
    bug_stack::BugStack,
    color::Color,
    dfs_info::DfsInfo,
    direction::Direction,
    game_error::GameError,
    game_result::GameResult,
    game_type::GameType,
    piece::Piece,
    position::Position,
    torus_array::TorusArray,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fmt::{self, Write},
};

#[cfg(feature = "cli")]
use {
    crate::SvgPosition,
    anyhow::Result,
    std::{
        fs::{self, OpenOptions},
        io::{BufWriter, Write as Writer},
        path::PathBuf,
    },
};

pub const BOARD_SIZE: i32 = 32;
const MISSING_DFS_INDEX: u8 = u8::MAX;
lazy_static! {
    static ref BLACK_QUEEN: Piece = Piece::new_from(Bug::Queen, Color::Black, 0);
    static ref WHITE_QUEEN: Piece = Piece::new_from(Bug::Queen, Color::White, 0);
}

/// Acts as a more transparent representation of
/// locations and stacks pieces on the board.
/// Each stack is arranged from lowest to highest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stacks {
    positions: HashMap<Position, Vec<Piece>>,
}

impl Default for Stacks {
    fn default() -> Self {
        Self::new()
    }
}

impl Stacks {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }
    /// Given an odd-r offset coordinate, return the pieces (if any)
    /// at that location.
    ///
    /// The vector of pieces returned represents a Hive stack arranged from
    /// lowest (first element) to highest.
    pub fn get(&self, q: i32, r: i32) -> Vec<Piece> {
        let position = Position { q, r };
        self.positions.get(&position).unwrap_or(&Vec::new()).clone()
    }

    pub fn get_ref(&self, q: i32, r: i32) -> &[Piece] {
        self.positions
            .get(&Position { q, r })
            .map_or(&[], Vec::as_slice)
    }
}

#[derive(Clone, Debug)]
pub struct Bounds {
    top_left: Position,
    bottom_right: Position,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            top_left: Position::new(0, 0),
            bottom_right: Position::new(0, 0),
        }
    }
}

impl Bounds {
    pub fn top(&self) -> i32 {
        self.top_left.r
    }

    pub fn bottom(&self) -> i32 {
        self.bottom_right.r
    }

    pub fn left(&self) -> i32 {
        self.top_left.q
    }

    pub fn right(&self) -> i32 {
        self.bottom_right.q
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct Board {
    pub board: TorusArray<BugStack>,
    pub neighbor_count: TorusArray<u8>,
    // last moved contains the piece that was last moved
    pub last_moved: Option<(Piece, Position)>,
    // last move contains a from and to position of the last move
    pub last_move: (Option<Position>, Option<Position>),
    pub stunned: Option<Piece>,
    pub positions: [Option<Position>; 48],
    pinned: [bool; 48],
    // number of pieces present on the board
    pub played: usize,
}

/// Storage size is not part of the hive; derived fields follow from the compared ones.
impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.played == other.played
            && self.last_moved == other.last_moved
            && self.last_move == other.last_move
            && self.stunned == other.stunned
            && self.positions == other.positions
            && self.pinned == other.pinned
            && self
                .all_taken_positions()
                .all(|at| self.board.get(at) == other.board.get(at))
    }
}

impl Eq for Board {}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Everything needed to put a board back exactly as it was.
pub struct BoardSnapshot {
    pieces: Vec<(Position, Piece)>,
    last_moved: Option<(Piece, Position)>,
    last_move: (Option<Position>, Option<Position>),
    stunned: Option<Piece>,
    pinned: [bool; 48],
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
        }
    }

    pub fn storage_cells(&self) -> usize {
        self.board.cells()
    }

    /// One-way; shrinking happens by rebuilding in [`Self::recenter`].
    pub fn grow_storage(&mut self) {
        self.board.grow();
        self.neighbor_count.grow();
    }

    /// An insert touches its neighbour ring too; grow before anything aliases.
    fn ensure_storage_for(&mut self, position: Position) {
        const INNER: std::ops::RangeInclusive<i32> =
            (crate::torus_array::SMALL_OFFSET + 1)..=(crate::torus_array::SMALL_OFFSET + 14);
        if self.board.is_small() && (!INNER.contains(&position.q) || !INNER.contains(&position.r)) {
            self.grow_storage();
        }
    }

    /// `Board::all_positions` would visit cells small storage cannot address.
    pub fn scan_positions(&self) -> impl Iterator<Item = Position> {
        let (start, end) = if self.board.is_small() {
            (
                crate::torus_array::SMALL_OFFSET,
                crate::torus_array::SMALL_OFFSET + crate::torus_array::SMALL_SIZE,
            )
        } else {
            (0, BOARD_SIZE)
        };
        (start..end)
            .cartesian_product(start..end)
            .map(|(q, r)| Position { q, r })
    }

    /// Translate the hive back to the middle, unwrapped whole across the seam. Hashes and
    /// notation are translation-invariant; only raw coordinates - the renderer's input - move.
    pub fn recenter(&mut self) {
        if self.played == 0 {
            return;
        }
        let (mut q_mask, mut r_mask) = (0u32, 0u32);
        for position in self.positions.iter().flatten() {
            q_mask |= 1 << position.q;
            r_mask |= 1 << position.r;
        }
        let (q_origin, r_origin) = (
            crate::canonical_hash::axis_origin(q_mask),
            crate::canonical_hash::axis_origin(r_mask),
        );
        let (mut q_width, mut r_width) = (0, 0);
        for position in self.positions.iter().flatten() {
            q_width = q_width.max((position.q - q_origin).rem_euclid(BOARD_SIZE));
            r_width = r_width.max((position.r - r_origin).rem_euclid(BOARD_SIZE));
        }
        let centre = Position::initial_spawn_position();
        // A hive up to 12 wide (delta 11) fits the small window with its probe margins.
        let fits_small = q_width.max(r_width) <= 11;
        let (q_start, r_start) = if fits_small {
            (
                (centre.q - q_width / 2).clamp(10, 21 - q_width),
                (centre.r - r_width / 2).clamp(10, 21 - r_width),
            )
        } else {
            (centre.q - q_width / 2, centre.r - r_width / 2)
        };
        let translate = |at: Position| {
            Position::new(
                q_start + (at.q - q_origin).rem_euclid(BOARD_SIZE),
                r_start + (at.r - r_origin).rem_euclid(BOARD_SIZE),
            )
        };
        let mut centered = Board::new();
        if !fits_small {
            centered.grow_storage();
        }
        for at in self.all_taken_positions() {
            let stack = self.board.get(at);
            for piece in &stack.pieces[..stack.len()] {
                centered.insert(translate(at), *piece, true);
            }
        }
        centered.last_moved = self.last_moved.map(|(piece, at)| (piece, translate(at)));
        centered.last_move = (
            self.last_move.0.map(translate),
            self.last_move.1.map(translate),
        );
        centered.stunned = self.stunned;
        *self = centered;
    }

    /// A freshly recentered hive answers false, so recentering cannot retrigger itself.
    pub fn needs_recentering(&self) -> bool {
        if self.played == 0 {
            return false;
        }
        let comfort = if self.board.is_small() {
            10..=21
        } else {
            2..=BOARD_SIZE - 2
        };
        let (mut q_min, mut q_max, mut r_min, mut r_max) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        let mut outside = false;
        for p in self.positions.iter().flatten() {
            outside |= !comfort.contains(&p.q) || !comfort.contains(&p.r);
            q_min = q_min.min(p.q);
            q_max = q_max.max(p.q);
            r_min = r_min.min(p.r);
            r_max = r_max.max(p.r);
        }
        // Shrink as soon as the hive fits - storage must be a function of the hive alone,
        // or a snapshot restore recenters on a different cadence than live play.
        let fits_small = (q_max - q_min).max(r_max - r_min) <= 11;
        outside || self.board.is_small() != fits_small
    }

    pub fn snapshot(&self) -> BoardSnapshot {
        let mut pieces = Vec::with_capacity(self.played);
        for position in self.all_taken_positions() {
            let stack = self.board.get(position);
            for piece in &stack.pieces[..stack.len()] {
                pieces.push((position, *piece));
            }
        }
        BoardSnapshot {
            pieces,
            last_moved: self.last_moved,
            last_move: self.last_move,
            stunned: self.stunned,
            pinned: self.pinned,
        }
    }

    pub fn from_snapshot(snapshot: &BoardSnapshot) -> Self {
        let mut board = Self::new();
        // Size the storage as live play would; `ensure_storage_for` stays as the hostile net.
        let (mut q_min, mut q_max, mut r_min, mut r_max) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for (position, _) in &snapshot.pieces {
            q_min = q_min.min(position.q);
            q_max = q_max.max(position.q);
            r_min = r_min.min(position.r);
            r_max = r_max.max(position.r);
        }
        if !snapshot.pieces.is_empty() && (q_max - q_min).max(r_max - r_min) > 11 {
            board.grow_storage();
        }
        for (position, piece) in &snapshot.pieces {
            board.ensure_storage_for(*position);
            if board.board.get(*position).is_empty() {
                board.neighbor_count_add(*position);
            }
            board.board.get_mut(*position).push_piece(*piece);
            board.set_position_of_piece(*piece, *position);
        }
        board.last_moved = snapshot.last_moved;
        board.last_move = snapshot.last_move;
        board.stunned = snapshot.stunned;
        board.pinned = snapshot.pinned;
        board.played = snapshot.pieces.len();
        board
    }

    #[cfg(feature = "cli")]
    pub fn create_svg(&self, mut path: PathBuf) -> Result<()> {
        path.set_extension("svg");
        let file = OpenOptions::new()
            .read(true)
            .write(true) // Required for creation
            .create(true)
            .truncate(true)
            .open(&path)?;
        let mut writer = BufWriter::new(file);

        let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        let mut positions_piece = Vec::new();

        for (offset, maybe_position) in self.positions.iter().enumerate() {
            if let Some(position) = maybe_position {
                let piece = self.offset_to_piece(offset);
                let level = self.level_of_piece(piece, *position).unwrap_or(0);
                let center = SvgPosition::center_for_level(*position, level, true);
                if center.0 < min_x {
                    min_x = center.0;
                }
                if center.0 > max_x {
                    max_x = center.0;
                }
                if center.1 < min_y {
                    min_y = center.1;
                }
                if center.1 > max_y {
                    max_y = center.1;
                }
                positions_piece.push((position, piece));
            }
        }

        positions_piece.sort_by(|(pos_a, piece_a), (pos_b, piece_b)| {
            let level_a = self.level_of_piece(*piece_a, **pos_a).unwrap_or(0);
            let level_b = self.level_of_piece(*piece_b, **pos_b).unwrap_or(0);
            if level_a != level_b {
                level_a.cmp(&level_b)
            } else if pos_a.r != pos_b.r {
                pos_a.r.cmp(&pos_b.r)
            } else {
                pos_a.q.cmp(&pos_b.q)
            }
        });

        let piece_height = 104.242;
        let piece_width = 88.338;
        let space_around = 20.0;
        let width = max_x - min_x + piece_width + 2.0 * space_around;
        let height = max_y - min_y + piece_height + 2.0 * space_around;

        writeln!(
            writer,
            "<svg width=\"{width}\" height=\"{height}\" xmlns=\"http://www.w3.org/2000/svg\">"
        )?;
        let pieces = std::fs::read_to_string("pieces.svg")?;
        writeln!(writer, "{pieces}")?;
        writeln!(
            writer,
            "<g transform=\"translate({} {})\">",
            -min_x + space_around,
            -min_y + space_around
        )?;

        for (position, piece) in positions_piece.iter() {
            let level = self.level_of_piece(*piece, **position).unwrap_or(0);
            let center = SvgPosition::center_for_level(**position, level, true);
            // TODO: @leex add dropshadow

            let dot_color = match piece.bug() {
                Bug::Ant => "#289ee0",
                Bug::Beetle => "#9a7fc7",
                Bug::Grasshopper => "#42b23c",
                Bug::Spider => "#a4572a",
                _ => "#FF0000",
            };

            writeln!(writer, "  <g>")?;
            writeln!(
                writer,
                "    <use href=\"#{}\" x=\"{}\" y=\"{}\"></use>",
                piece.color().name(),
                center.0,
                center.1
            )?;
            writeln!(
                writer,
                "    <use href=\"#{}{}\" x=\"{}\" y=\"{}\"></use>",
                piece.color().name(),
                piece.bug().name(),
                center.0,
                center.1
            )?;
            // dots
            if piece.order() > 0 {
                writeln!(writer, "  <g fill=\"{dot_color}\">")?;
                writeln!(
                    writer,
                    "    <use href=\"#a{}\" x=\"{}\" y=\"{}\"></use>",
                    piece.order(),
                    center.0,
                    center.1
                )?;
                writeln!(writer, "  </g>")?;
            }
            // writeln!(
            //     writer,
            //     "    <use href=\"#shadow\" x=\"{}\" y=\"{}\"></use>",
            //     center.0, center.1
            // )?;
            writeln!(writer, "  </g>")?;
        }
        writeln!(writer, "</g>")?;
        writeln!(writer, "</svg>")?;
        writer.flush()?;

        let text = fs::read_to_string(&path)?;

        let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default())?;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(tree.size().width() as u32, tree.size().height() as u32)
                .unwrap();
        // Render SVG to pixmap
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );

        path.set_extension("png");
        pixmap.save_png(path)?;
        Ok(())
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
        panic!("{self} Finding From position: {from} to piece: {to} failed.");
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
        if !matches!(self.game_result(), GameResult::Unknown) {
            return false;
        }
        if self.played < 3 || !self.queen_played(color) {
            return false;
        }
        if self.can_spawn_from_reserve(color, game_type) {
            return false;
        }
        !self.has_board_action(color)
    }

    fn can_spawn_from_reserve(&self, color: Color, game_type: GameType) -> bool {
        if self.played == game_type.max_played() {
            return false;
        }
        self.has_reserve_piece(color, game_type) && self.has_spawnable_position(color)
    }

    fn has_reserve_piece(&self, color: Color, game_type: GameType) -> bool {
        let start = 24 * color as usize;
        let end = 24 + start;
        self.positions[start..end]
            .iter()
            .enumerate()
            .any(|(i, maybe_pos)| {
                let offset = i + start;
                maybe_pos.is_none() && self.offset_represents_piece(offset, game_type)
            })
    }

    fn has_spawnable_position(&self, color: Color) -> bool {
        if self.played < 2 {
            return true;
        }
        // Spawn candidates must border one of our top pieces, so avoid scanning the full board.
        self.top_pieces().any(|(piece, pos)| {
            if !piece.is_color(color) {
                return false;
            }
            pos.positions_around().any(|target| {
                !self.occupied(target)
                    && !self
                        .top_layer_neighbors(target)
                        .any(|piece| color == piece.color().opposite_color())
            })
        })
    }

    fn has_board_action(&self, color: Color) -> bool {
        for (piece, pos) in self.top_pieces() {
            if !piece.is_color(color) || self.last_moved == Some((piece, pos)) {
                continue;
            }
            if !self.is_pinned(piece) && Bug::has_move(pos, self) {
                return true;
            }
            if Bug::has_available_ability(pos, self) {
                return true;
            }
        }
        false
    }

    pub fn game_result(&self) -> GameResult {
        let black_won = self
            .position_of_piece(*WHITE_QUEEN)
            .map(|pos| *self.neighbor_count.get(pos) == 6);
        let white_won = self
            .position_of_piece(*BLACK_QUEEN)
            .map(|pos| *self.neighbor_count.get(pos) == 6);
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

    /// @neal - move piece
    /// `mover` is the colour whose turn it is, which is not always `piece.color()`: a Pillbug can
    /// move an *enemy* piece. [`Board::set_stunned`] needs to know which it was.
    pub fn move_piece(
        &mut self,
        piece: Piece,
        current: Position,
        target: Position,
        turn: usize,
        mover: Color,
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

        let ground_graph_changes = self.level(current) == 1 || !self.occupied(target);
        let removed_piece = self.remove(current);
        debug_assert_eq!(removed_piece, piece);
        self.insert_with_pinned_update(target, piece, false, ground_graph_changes, mover);
        Ok(())
    }

    /// @neal - remove piece
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

    pub fn offset_to_piece(&self, offset: usize) -> Piece {
        let color = offset as u8 / 24;
        let bug = (offset as u8 - color * 24) / 3;
        let order = (offset as u8 + 1 - bug * 3 - color * 24) as usize;
        Piece::new_from(Bug::from(bug), Color::from(color), order)
    }

    fn offset_represents_piece(&self, offset: usize, game_type: GameType) -> bool {
        self.offset_to_piece(offset).bug().count(game_type) > offset % 3
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
        match self.game_result() {
            GameResult::Unknown => {}
            _ => return false,
        }
        if !self.queen_played(color) {
            return false;
        }
        if self.top_piece(current_position) != Some(piece) {
            return false;
        }
        if self.last_moved == Some((piece, current_position)) {
            return false;
        }

        if piece.is_color(color)
            && !self.is_pinned(piece)
            && Bug::has_target_move(current_position, target_position, self)
        {
            return true;
        }

        for (_, ability_position) in self.ability_pieces_around(color, current_position) {
            if Bug::can_throw_piece_to(ability_position, current_position, target_position, self) {
                return true;
            }
        }

        false
    }

    fn ability_pieces_around(
        &self,
        color: Color,
        position: Position,
    ) -> impl Iterator<Item = (Piece, Position)> + '_ {
        position.positions_around().filter_map(move |pos| {
            let piece = self.top_piece(pos)?;
            if !piece.is_color(color) || self.last_moved == Some((piece, pos)) {
                return None;
            }
            match piece.bug() {
                Bug::Pillbug => Some((piece, pos)),
                Bug::Mosquito if self.level(pos) == 1 && self.neighbor_is_a(pos, Bug::Pillbug) => {
                    Some((piece, pos))
                }
                _ => None,
            }
        })
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
        for (piece, pos) in self.top_pieces() {
            if !piece.is_color(color) || self.last_moved == Some((piece, pos)) {
                continue;
            }
            for (start_pos, mut target_positions) in Bug::available_moves(pos, self) {
                if let Some(piece) = self.top_piece(start_pos) {
                    if !target_positions.is_empty() {
                        moves
                            .entry((piece, start_pos))
                            .or_default()
                            .append(&mut target_positions);
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
        let game_result = self.game_result();
        // Only an empty board needs the seed; later it duplicates a negative-space cell
        // whenever the spawn position is empty but touches the hive.
        let seed = (self.played == 0).then(Position::initial_spawn_position);
        seed.into_iter()
            .chain(self.negative_space())
            .filter(move |pos| self.spawnable_with_game_result(color, *pos, &game_result))
    }

    pub fn queen_played(&self, color: Color) -> bool {
        self.piece_already_played(Piece::new_from(Bug::Queen, color, 0))
    }

    /// A side that has not played its queen has never moved or passed, so its pieces on the
    /// board are exactly its turns taken - true of a loaded position as much as a played one.
    pub fn queen_required(&self, color: Color) -> bool {
        !self.queen_played(color) && self.played_by(color) == 3
    }

    pub fn played_by(&self, color: Color) -> usize {
        let start = 24 * color as usize;
        self.positions[start..start + 24]
            .iter()
            .filter(|position| position.is_some())
            .count()
    }

    pub fn update_pinned(&mut self) {
        self.calculate_pinned().into_iter().for_each(|pinned_info| {
            let offset = self.piece_to_offset(pinned_info.piece);
            self.pinned[offset] = pinned_info.pinned;
        });
    }

    pub fn calculate_pinned(&self) -> Vec<DfsInfo> {
        // Connectivity is position-based, so stacked positions contribute only their bottom piece.
        let mut dfs_info = Vec::with_capacity(self.played);
        // Indexed at occupied cells, so it must cover the board's window.
        let mut dfs_indexes = TorusArray::new_like(&self.board, MISSING_DFS_INDEX);

        for (i, maybe_pos) in self.positions.iter().enumerate() {
            let Some(pos) = maybe_pos else {
                continue;
            };
            let piece = self.offset_to_piece(i);

            if self.is_bottom_piece(piece, *pos) {
                let dfs_index = dfs_info.len();
                debug_assert!(dfs_index < usize::from(MISSING_DFS_INDEX));

                dfs_indexes.set(*pos, dfs_index as u8);
                dfs_info.push(DfsInfo {
                    position: *pos,
                    piece,
                    visited: false,
                    depth: 0,
                    low: 0,
                    pinned: false,
                    parent: None,
                });
            }
        }

        if dfs_info.is_empty() {
            return dfs_info;
        }
        self.mark_articulation_points(0, 0, &mut dfs_info, &dfs_indexes);
        dfs_info
    }

    fn mark_articulation_points(
        &self,
        index: usize,
        depth: usize,
        dfs_info: &mut [DfsInfo],
        dfs_indexes: &TorusArray<u8>,
    ) {
        dfs_info[index].visited = true;
        dfs_info[index].depth = depth;
        dfs_info[index].low = depth;
        let mut child_count = 0;
        let mut is_articulation_point = false;

        for pos in self.positions_taken_around(dfs_info[index].position) {
            let neighbor_dfs_index = usize::from(*dfs_indexes.get(pos));
            debug_assert!(
                neighbor_dfs_index < dfs_info.len(),
                "Occupied position should have a DFS index"
            );

            if !dfs_info[neighbor_dfs_index].visited {
                child_count += 1;
                dfs_info[neighbor_dfs_index].parent = Some(index);
                self.mark_articulation_points(neighbor_dfs_index, depth + 1, dfs_info, dfs_indexes);
                if dfs_info[neighbor_dfs_index].low >= dfs_info[index].depth {
                    is_articulation_point = true;
                }
                dfs_info[index].low =
                    std::cmp::min(dfs_info[index].low, dfs_info[neighbor_dfs_index].low);
            } else {
                let is_alternate_connection = dfs_info[index]
                    .parent
                    .is_some_and(|parent_dfs_index| neighbor_dfs_index != parent_dfs_index);
                if is_alternate_connection {
                    dfs_info[index].low =
                        std::cmp::min(dfs_info[index].low, dfs_info[neighbor_dfs_index].depth);
                }
            }
        }

        if dfs_info[index].parent.is_none() {
            is_articulation_point = child_count > 1;
        }
        dfs_info[index].pinned = is_articulation_point;
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
        for (i, maybe_pos) in self.positions[start..end].iter().enumerate() {
            if maybe_pos.is_none() {
                let offset = i + start;
                if self.offset_represents_piece(offset, game_type) {
                    let piece = self.offset_to_piece(offset);
                    res.entry(piece.bug()).or_default().push(piece.to_string());
                }
            }
        }
        res
    }

    pub fn all_taken_positions(&self) -> impl Iterator<Item = Position> + '_ {
        self.top_pieces().map(|(_, position)| position)
    }

    fn top_pieces(&self) -> impl Iterator<Item = (Piece, Position)> + '_ {
        self.positions
            .iter()
            .enumerate()
            .filter_map(|(offset, maybe_position)| {
                let position = (*maybe_position)?;
                let piece = self.offset_to_piece(offset);
                (self.top_piece(position) == Some(piece)).then_some((piece, position))
            })
    }

    pub fn center_coordinates(&self) -> Position {
        let mut positions = 0;
        let (q_min, q_max, r_min, r_max) = self.all_taken_positions().fold(
            (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
            |(q_min, q_max, r_min, r_max), pos| {
                positions += 1;
                (
                    q_min.min(pos.q),
                    q_max.max(pos.q),
                    r_min.min(pos.r),
                    r_max.max(pos.r),
                )
            },
        );
        //center won't shift much if any in the first few moves
        if positions < 8 {
            return Position::initial_spawn_position();
        }

        //TODO: Some look centered with q + 1 some without it, figure out something
        Position {
            q: q_min + ((q_max - q_min) / 2),
            r: r_min + ((r_max - r_min) / 2),
        }
    }

    pub fn spawnable(&self, color: Color, position: Position) -> bool {
        self.spawnable_with_game_result(color, position, &self.game_result())
    }

    fn spawnable_with_game_result(
        &self,
        color: Color,
        position: Position,
        game_result: &GameResult,
    ) -> bool {
        if !matches!(game_result, GameResult::Unknown) {
            return false;
        }
        if self.occupied(position) {
            return false;
        }
        if self.played == 0 {
            return position == Position::initial_spawn_position();
        }
        if self.played == 1 {
            return self.is_negative_space(position);
        }

        let mut neighbors = self.top_layer_neighbors(position).peekable();
        if neighbors.peek().is_none() {
            return false;
        }
        !neighbors.any(|piece| color == piece.color().opposite_color())
    }

    pub fn negative_space(&self) -> impl Iterator<Item = Position> + '_ {
        self.scan_positions()
            .filter(move |pos| self.is_negative_space(*pos))
    }

    pub fn is_negative_space(&self, position: Position) -> bool {
        !self.occupied(position) && *self.neighbor_count.get(position) > 0
    }

    /// Stun only when the restriction removes a legal move - it feeds the hash, and a vacuous
    /// stun splits identical positions. `mover` != `piece.color()` when a Pillbug throws.
    pub fn set_stunned(&mut self, position: Position, piece: Piece, spawn: bool, mover: Color) {
        // A spawn never touches an enemy piece.
        if spawn {
            self.stunned = None;
            return;
        }
        // Stacked pieces cannot be thrown, and a throw lands on empty cells.
        if self.level(position) > 1 {
            self.stunned = None;
            return;
        }
        // A decided game has no moves left to restrict (threefolds end at State level).
        if self.game_result() != GameResult::Unknown {
            self.stunned = None;
            return;
        }
        let opponent = mover.opposite_color();
        // No board moves before the Queen is down.
        if !self.queen_played(opponent) {
            self.stunned = None;
            return;
        }

        // A thrown opponent piece may not act at all, so movement and ability are separate losses.
        let is_the_opponents_piece = piece.color() == opponent;
        let loses_its_own_move =
            is_the_opponents_piece && !self.is_pinned(piece) && Bug::has_move(position, self);
        // Not behind `is_pinned`: the ability moves another piece, so a pinned Pillbug keeps it.
        let loses_its_own_ability =
            is_the_opponents_piece && Bug::has_available_ability(position, self);

        // Otherwise only the throw itself is lost - if the opponent could actually have made it.
        let loses_a_throw = || {
            self.ability_pieces_around(opponent, position)
                .any(|(_, ability)| Bug::could_throw_ignoring_last_moved(ability, position, self))
        };

        self.stunned =
            (loses_its_own_move || loses_its_own_ability || loses_a_throw()).then_some(piece);
    }

    /// @neal - add piece
    pub fn insert(&mut self, position: Position, piece: Piece, spawn: bool) {
        // You can only ever place your own piece, so for a spawn the mover is the piece owner.
        self.insert_with_pinned_update(position, piece, spawn, true, piece.color());
    }

    fn insert_with_pinned_update(
        &mut self,
        position: Position,
        piece: Piece,
        spawn: bool,
        update_pinned: bool,
        mover: Color,
    ) {
        self.ensure_storage_for(position);
        self.last_moved = Some((piece, position));
        let stack = self.board.get_mut(position);
        stack.push_piece(piece);
        self.set_position_of_piece(piece, position);
        if self.board.get(position).size == 1 {
            self.neighbor_count_add(position)
        }
        if update_pinned {
            self.update_pinned();
        }
        if spawn {
            self.played += 1;
        }
        self.set_stunned(position, piece, spawn, mover);
    }

    pub fn all_positions() -> impl Iterator<Item = Position> {
        (0..BOARD_SIZE)
            .cartesian_product(0..BOARD_SIZE)
            .map(|(q, r)| Position { q, r })
    }

    pub fn bounds(&self) -> Option<Bounds> {
        if self.played == 0 {
            return None;
        }

        let (top_left, bottom_right) = self.all_taken_positions().fold(
            (
                Position {
                    q: BOARD_SIZE,
                    r: BOARD_SIZE,
                },
                Position::new(0, 0),
            ),
            |(top_left, bottom_right), pos| {
                (
                    Position {
                        q: top_left.q.min(pos.q),
                        r: top_left.r.min(pos.r),
                    },
                    Position {
                        q: bottom_right.q.max(pos.q),
                        r: bottom_right.r.max(pos.r),
                    },
                )
            },
        );

        Some(Bounds {
            top_left,
            bottom_right,
        })
    }

    pub fn stacks(&self) -> Stacks {
        let mut stacks = Stacks::new();
        for (i, maybe_pos) in self.positions.iter().enumerate() {
            if let Some(pos) = maybe_pos {
                let entry = stacks.positions.entry(*pos).or_default();
                entry.push(self.offset_to_piece(i));
            }
        }

        for (pos, pieces) in stacks.positions.iter_mut() {
            pieces.sort_by(|a, b| {
                self.level_of_piece(*a, *pos)
                    .cmp(&self.level_of_piece(*b, *pos))
            });
        }
        stacks
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
    use crate::{game_status::GameStatus, history::History, state::State};
    use std::collections::HashSet;

    fn queens_under_siege(surrounded: &[Color]) -> Board {
        let mut board = Board::new();
        let (white_at, black_at) = (Position::new(16, 16), Position::new(17, 16));
        board.insert(white_at, "wQ".parse().expect("test piece"), true);
        board.insert(black_at, "bQ".parse().expect("test piece"), true);
        let mut fillers = [
            "wA1", "wA2", "wA3", "wG1", "wG2", "wG3", "wS1", "wS2", "bA1", "bA2", "bA3", "bG1",
            "bG2", "bG3",
        ]
        .into_iter();
        for (queen, color) in [(white_at, Color::White), (black_at, Color::Black)] {
            let ring: Vec<Position> = queen
                .positions_around()
                .filter(|p| board.top_piece(*p).is_none())
                .collect();
            let take = if surrounded.contains(&color) {
                ring.len()
            } else {
                ring.len() - 1
            };
            for position in ring.into_iter().take(take) {
                let piece = fillers.next().expect("enough fillers");
                board.insert(position, piece.parse().expect("test piece"), true);
            }
        }
        board
    }

    /// The corpus checks this over 104k real games, but only when the CSVs are on hand.
    #[test]
    fn a_surrounded_queen_loses_for_its_owner() {
        assert_eq!(
            queens_under_siege(&[Color::White]).game_result(),
            GameResult::Winner(Color::Black)
        );
        assert_eq!(
            queens_under_siege(&[Color::Black]).game_result(),
            GameResult::Winner(Color::White)
        );
    }

    #[test]
    fn the_game_is_a_draw_only_when_both_queens_are_surrounded() {
        assert_eq!(
            queens_under_siege(&[Color::White, Color::Black]).game_result(),
            GameResult::Draw
        );
        assert_eq!(queens_under_siege(&[]).game_result(), GameResult::Unknown);
    }

    #[test]
    fn the_queen_comes_due_on_the_fourth_placement() {
        let mut board = Board::new();
        for (offset, piece) in ["wA1", "wA2", "wA3"].into_iter().enumerate() {
            assert!(!board.queen_required(Color::White));
            board.insert(
                Position::new(16 + offset as i32, 16),
                piece.parse().expect("test piece"),
                true,
            );
        }
        assert!(board.queen_required(Color::White));
        assert!(!board.queen_required(Color::Black));

        board.insert(
            Position::new(19, 16),
            "wQ".parse().expect("test piece"),
            true,
        );
        assert!(!board.queen_required(Color::White));
    }

    /// The diet's premise: the middle window is all an opening needs.
    #[test]
    fn a_fresh_board_uses_small_storage() {
        assert_eq!(Board::new().storage_cells(), 256);
    }

    /// Direct construction outside the small window must grow, not alias.
    #[test]
    fn an_out_of_window_insert_grows_the_storage() {
        let mut board = Board::new();
        board.insert(
            Position::new(30, 16),
            "wQ".parse().expect("test piece"),
            true,
        );
        assert_eq!(board.storage_cells(), 1024);
        assert!(board.top_piece(Position::new(30, 16)).is_some());
    }

    /// Equality has to be able to say "no", and on every field it compares - the snapshot
    /// oracles are `assert_eq!` on boards, so an equality that cannot fail proves nothing.
    #[test]
    fn boards_that_differ_anywhere_are_not_equal() {
        let mut base = Board::new();
        base.insert(
            Position::new(16, 16),
            "wQ".parse().expect("test piece"),
            true,
        );
        base.insert(
            Position::new(17, 16),
            "bQ".parse().expect("test piece"),
            true,
        );
        base.last_moved = Some(("bQ".parse().expect("test piece"), Position::new(17, 16)));
        base.last_move = (Some(Position::new(17, 15)), Some(Position::new(17, 16)));
        base.stunned = Some("wQ".parse().expect("test piece"));

        let mut moved = base.clone();
        moved.last_moved = Some(("bQ".parse().expect("test piece"), Position::new(16, 15)));
        assert_ne!(base, moved, "last_moved must count");

        let mut travelled = base.clone();
        travelled.last_move = (Some(Position::new(15, 15)), Some(Position::new(17, 16)));
        assert_ne!(base, travelled, "last_move must count");

        let mut unstunned = base.clone();
        unstunned.stunned = None;
        assert_ne!(base, unstunned, "stunned must count");

        let mut pinned = base.clone();
        pinned.pinned[0] = !pinned.pinned[0];
        assert_ne!(base, pinned, "pinned must count");

        let mut relocated = base.clone();
        relocated.positions[0] = Some(Position::new(20, 20));
        assert_ne!(base, relocated, "positions must count");

        let mut extra = base.clone();
        extra.insert(
            Position::new(18, 16),
            "bA1".parse().expect("test piece"),
            true,
        );
        assert_ne!(base, extra, "an extra piece must count");

        // A stack differing only above the top piece: same positions, different contents.
        let mut stacked = base.clone();
        stacked
            .board
            .get_mut(Position::new(16, 16))
            .push_piece("bB1".parse().expect("test piece"));
        assert_ne!(base, stacked, "stack contents must count");
    }

    #[test]
    fn equality_ignores_the_storage_size() {
        let mut small = Board::new();
        small.insert(
            Position::new(16, 16),
            "wQ".parse().expect("test piece"),
            true,
        );
        small.insert(
            Position::new(17, 16),
            "bQ".parse().expect("test piece"),
            true,
        );
        let mut grown = small.clone();
        grown.grow_storage();
        assert_eq!(small.storage_cells(), 256);
        assert_eq!(grown.storage_cells(), 1024);
        assert_eq!(small, grown);
        assert_eq!(Board::from_snapshot(&grown.snapshot()), small);
    }

    /// The clamp keeps wrong placements inside the window, so assert where the hive lands, not
    /// just that it fits. Both axes get extent, or one axis only ever runs at width zero.
    #[test]
    fn recentering_puts_the_hive_in_the_middle() {
        let centre = Position::initial_spawn_position();
        for (q_len, r_len) in [(1usize, 1usize), (5, 1), (1, 5), (4, 3), (3, 8)] {
            let mut board = Board::new();
            let pieces = [
                "wA1", "wA2", "wA3", "wG1", "wG2", "wG3", "wS1", "wS2", "bA1", "bA2", "bA3", "bG1",
                "bG2",
            ];
            let cells = (0..q_len)
                .map(|q| (q, 0))
                .chain((1..r_len).map(|r| (0, r)))
                .collect::<Vec<_>>();
            for ((q, r), piece) in cells.into_iter().zip(pieces) {
                board.insert(
                    Position::new(11 + q as i32, 12 + r as i32),
                    piece.parse().expect("test piece"),
                    true,
                );
            }
            board.recenter();
            let (q_min, q_max) = (
                board
                    .all_taken_positions()
                    .map(|p| p.q)
                    .min()
                    .expect("hive"),
                board
                    .all_taken_positions()
                    .map(|p| p.q)
                    .max()
                    .expect("hive"),
            );
            let (r_min, r_max) = (
                board
                    .all_taken_positions()
                    .map(|p| p.r)
                    .min()
                    .expect("hive"),
                board
                    .all_taken_positions()
                    .map(|p| p.r)
                    .max()
                    .expect("hive"),
            );
            // Integer halving lands the box on the centre or one cell before it.
            assert!(
                (centre.q - q_min) - (q_max - centre.q) <= 1
                    && (q_max - centre.q) - (centre.q - q_min) <= 1,
                "hive {q_len}x{r_len} is off-centre on q: {q_min}..={q_max}"
            );
            assert!(
                (centre.r - r_min) - (r_max - centre.r) <= 1
                    && (r_max - centre.r) - (centre.r - r_min) <= 1,
                "hive {q_len}x{r_len} is off-centre on r: {r_min}..={r_max}"
            );
        }
    }

    /// Shrink on recenter once the hive fits again, so play, undo, and restore agree.
    #[test]
    fn recentering_shrinks_a_grown_board_once_the_hive_fits_again() {
        let mut board = Board::new();
        board.grow_storage();
        board.insert(
            Position::new(16, 16),
            "wQ".parse().expect("test piece"),
            true,
        );
        board.insert(
            Position::new(17, 16),
            "bQ".parse().expect("test piece"),
            true,
        );
        assert!(board.needs_recentering());
        board.recenter();
        assert_eq!(board.storage_cells(), 256);
        assert!(!board.needs_recentering());
    }

    #[test]
    fn from_snapshot_matches_the_live_storage_size() {
        let mut board = Board::new();
        board.grow_storage();
        let pieces = [
            "wA1", "wA2", "wA3", "wB1", "wB2", "wG1", "wG2", "wG3", "wS1", "wS2", "wQ", "wM", "wL",
        ];
        for (piece, q) in pieces.iter().zip(9..=21) {
            board.insert(
                Position::new(q, 16),
                piece.parse().expect("test piece"),
                true,
            );
        }
        let restored = Board::from_snapshot(&board.snapshot());
        assert_eq!(restored.storage_cells(), 1024);
        assert_eq!(restored, board);
    }

    #[test]
    fn recenter_pulls_a_seam_straddling_hive_back_to_the_middle() {
        let mut board = Board::new();
        for (q, r, piece) in [
            (30, 16, "wQ"),
            (31, 16, "wA1"),
            (0, 16, "bQ"),
            (1, 16, "bA1"),
        ] {
            board.insert(
                Position::new(q, r),
                piece.parse().expect("test piece"),
                true,
            );
        }
        let before =
            crate::canonical_hash::canonical_hash(&board, crate::color::Color::White, None);
        board.recenter();
        for position in board.all_taken_positions() {
            assert!(
                (2..=30).contains(&position.q) && (2..=30).contains(&position.r),
                "still hugging the seam at {position}"
            );
        }
        assert_eq!(
            crate::canonical_hash::canonical_hash(&board, crate::color::Color::White, None),
            before,
            "recentering is a pure translation"
        );
    }

    fn assert_snapshot_equivalent(actual: &State, expected: &State) {
        // Board equality is by content, so no empty-stack normalisation is needed.
        assert_eq!(actual.board, expected.board);
        assert_eq!(*actual, *expected);
    }

    #[test]
    fn snapshot_restores_moved_stack_and_continues() {
        let mut history = History::from_filepath("./test_pgns/hash/short_pass.pgn".into())
            .expect("valid history");
        let continuation = history.moves[7].clone();
        assert_eq!(continuation, ("bL".to_string(), "-wQ".to_string()));
        history.moves.truncate(7);

        let mut expected = State::new_from_history(&history).expect("valid first seven plies");
        let beetle_position = expected
            .board
            .position_of_piece("wB1".parse().expect("valid piece"))
            .expect("beetle on board");
        assert_eq!(
            expected
                .board
                .position_of_piece("bQ".parse().expect("valid piece")),
            Some(beetle_position)
        );
        assert_eq!(expected.board.level(beetle_position), 2);

        let mut restored = expected.clone();
        restored.board = Board::from_snapshot(&expected.board.snapshot());
        assert_snapshot_equivalent(&restored, &expected);

        expected
            .play_turn_from_history(&continuation.0, &continuation.1)
            .expect("legal continuation");
        restored
            .play_turn_from_history(&continuation.0, &continuation.1)
            .expect("legal continuation after restore");
        assert_snapshot_equivalent(&restored, &expected);
    }

    #[test]
    fn tests_action_existence_matches_full_generation_for_pass_games() {
        for file in [
            "./test_pgns/valid/base_with_pass.pgn",
            "./test_pgns/valid/m_with_pass.pgn",
            "./test_pgns/valid/pass.pgn",
            "./test_pgns/valid/pass2.pgn",
        ] {
            let history = History::from_filepath(file.into()).expect("valid history");
            let tournament = !history.moves.iter().take(2).any(|(piece, _)| {
                piece
                    .parse::<Piece>()
                    .map(|piece| piece.bug() == Bug::Queen)
                    .unwrap_or(false)
            });
            let mut state = State::new(history.game_type, tournament);
            for (piece, position) in history.moves.iter() {
                let full_generation_has_legal_action =
                    matches!(state.board.game_result(), GameResult::Unknown)
                        && (!state.board.moves(state.turn_color).is_empty()
                            || (state
                                .board
                                .spawnable_positions(state.turn_color)
                                .next()
                                .is_some()
                                && !state
                                    .board
                                    .reserve(state.turn_color, state.game_type)
                                    .is_empty()));
                assert_eq!(
                    state.board.is_shutout(state.turn_color, state.game_type),
                    !full_generation_has_legal_action,
                    "{file} turn {}",
                    state.turn
                );
                state
                    .play_turn_from_history(piece, position)
                    .unwrap_or_else(|err| panic!("{file} turn {}: {err}", state.turn));
            }
        }
    }

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
    fn bounds_do_not_include_origin_when_all_pieces_are_offset() {
        let mut board = Board::new();
        board.insert(
            Position::new(4, 5),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        board.insert(
            Position::new(6, 7),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );

        let bounds = board.bounds().expect("occupied board has bounds");

        assert_eq!(bounds.left(), 4);
        assert_eq!(bounds.top(), 5);
        assert_eq!(bounds.right(), 6);
        assert_eq!(bounds.bottom(), 7);
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
    // stunned feeds the hash, so only set it when the restriction removes a legal move

    /// Three mutually adjacent cells, so nothing in the ring is pinned.
    fn triangle(centre: Position) -> (Position, Position, Position) {
        (centre, centre.to(Direction::E), centre.to(Direction::SE))
    }

    /// The colour test used to sit inside the Pillbug arm only (`&&` binds tighter than `||`),
    /// so a friendly Mosquito next to a Pillbug set a stun.
    #[test]
    fn set_stunned_ignores_a_friendly_ability_piece() {
        let (moved_at, mosquito_at, third) = triangle(Position::new(10, 10));
        let mut board = Board::new();
        let ant = Piece::new_from(Bug::Ant, Color::White, 1);
        board.insert(moved_at, ant, true);
        board.insert(
            mosquito_at,
            Piece::new_from(Bug::Mosquito, Color::White, 0),
            true,
        );
        board.insert(third, Piece::new_from(Bug::Pillbug, Color::White, 0), true);
        // Black needs a Queen on the board or the whole question is moot.
        board.insert(
            third.to(Direction::SE),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        assert!(board.queen_played(Color::Black));

        board.set_stunned(moved_at, ant, false, Color::White);
        assert_eq!(
            board.stunned, None,
            "White's own Mosquito cannot throw White's piece before the restriction lapses"
        );
    }

    /// A Mosquito that has climbed is a Beetle and borrows nothing. The legality code has always
    /// checked `level == 1`; `set_stunned` did not.
    #[test]
    fn set_stunned_ignores_a_climbed_mosquito() {
        let (moved_at, mosquito_at, third) = triangle(Position::new(10, 10));
        let mut board = Board::new();
        let ant = Piece::new_from(Bug::Ant, Color::White, 1);
        board.insert(moved_at, ant, true);
        board.insert(
            mosquito_at,
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        // Black's Mosquito climbs on top of its own Queen, so it moves as a Beetle from now on.
        board.insert(
            mosquito_at,
            Piece::new_from(Bug::Mosquito, Color::Black, 0),
            true,
        );
        board.insert(third, Piece::new_from(Bug::Pillbug, Color::Black, 0), true);
        assert_eq!(board.level(mosquito_at), 2);

        board.set_stunned(moved_at, ant, false, Color::White);
        assert_eq!(
            board.stunned, None,
            "a Mosquito on a stack has no Pillbug ability"
        );
    }

    /// Standing next to an enemy Pillbug is not a restriction if the Pillbug has nowhere to throw
    /// to. This one cost a real game its draw: hivegame.com/game/lYtE84YtiA_9.
    #[test]
    fn set_stunned_ignores_a_pillbug_with_nowhere_to_throw() {
        let moved_at = Position::new(10, 10);
        let pillbug_at = moved_at.to(Direction::E);
        let mut board = Board::new();
        let ant = Piece::new_from(Bug::Ant, Color::White, 1);
        board.insert(moved_at, ant, true);
        board.insert(
            pillbug_at,
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
            true,
        );
        // Ring the Pillbug so it has no empty cell left to throw into.
        let ring = [
            Piece::new_from(Bug::Grasshopper, Color::Black, 1),
            Piece::new_from(Bug::Grasshopper, Color::Black, 2),
            Piece::new_from(Bug::Grasshopper, Color::Black, 3),
            Piece::new_from(Bug::Spider, Color::Black, 1),
            // One of the ring is the Queen, or the restriction is moot for the wrong reason.
            Piece::new_from(Bug::Queen, Color::Black, 0),
        ];
        let mut filler = ring.into_iter();
        for around in pillbug_at.positions_around() {
            if around == moved_at {
                continue;
            }
            board.insert(around, filler.next().expect("five cells to fill"), true);
        }
        assert!(board.queen_played(Color::Black));

        board.set_stunned(moved_at, ant, false, Color::White);
        assert_eq!(
            board.stunned, None,
            "a fully surrounded Pillbug removes no legal move"
        );
    }

    /// Positive control: stops a "fix" that never stuns anything from passing every test above.
    #[test]
    fn set_stunned_marks_a_piece_the_opponent_could_throw() {
        let (moved_at, pillbug_at, third) = triangle(Position::new(10, 10));
        let mut board = Board::new();
        let ant = Piece::new_from(Bug::Ant, Color::White, 1);
        board.insert(moved_at, ant, true);
        board.insert(
            pillbug_at,
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
            true,
        );
        board.insert(third, Piece::new_from(Bug::Queen, Color::Black, 0), true);

        board.set_stunned(moved_at, ant, false, Color::White);
        assert_eq!(board.stunned, Some(ant));
    }

    /// A stun on a game-ending move made identical final positions hash apart over a restriction
    /// nobody can feel.
    #[test]
    fn set_stunned_ignores_a_restriction_once_the_game_is_over() {
        let (moved_at, pillbug_at, third) = triangle(Position::new(10, 10));
        let mut board = Board::new();
        let ant = Piece::new_from(Bug::Ant, Color::White, 1);
        board.insert(moved_at, ant, true);
        board.insert(
            pillbug_at,
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
            true,
        );
        board.insert(third, Piece::new_from(Bug::Queen, Color::Black, 0), true);

        // Surround the Black Queen: two of its neighbours are already taken by the triangle, so
        // four fillers finish the job and decide the game for White.
        let fillers = [
            Piece::new_from(Bug::Grasshopper, Color::White, 1),
            Piece::new_from(Bug::Grasshopper, Color::White, 2),
            Piece::new_from(Bug::Grasshopper, Color::White, 3),
            Piece::new_from(Bug::Spider, Color::White, 1),
        ];
        let mut filler = fillers.into_iter();
        for around in third.positions_around() {
            if board.occupied(around) {
                continue;
            }
            board.insert(around, filler.next().expect("four cells to fill"), true);
        }
        // `game_result` reports Unknown until both Queens are on the board.
        board.insert(
            third.to(Direction::SE).to(Direction::SE),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );
        assert_eq!(board.game_result(), GameResult::Winner(Color::White));

        board.set_stunned(moved_at, ant, false, Color::White);
        assert_eq!(
            board.stunned, None,
            "a decided game has no legal moves left to restrict"
        );
    }

    /// A piece that ended up on a stack cannot be thrown, and cannot have been the thrown piece
    /// either, because a throw always lands on an empty cell.
    #[test]
    fn set_stunned_ignores_a_stacked_piece() {
        let (moved_at, pillbug_at, third) = triangle(Position::new(10, 10));
        let mut board = Board::new();
        board.insert(moved_at, Piece::new_from(Bug::Queen, Color::White, 0), true);
        let beetle = Piece::new_from(Bug::Beetle, Color::White, 1);
        board.insert(moved_at, beetle, true);
        board.insert(
            pillbug_at,
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
            true,
        );
        board.insert(third, Piece::new_from(Bug::Queen, Color::Black, 0), true);

        board.set_stunned(moved_at, beetle, false, Color::White);
        assert_eq!(
            board.stunned, None,
            "a Beetle on top of a stack cannot be thrown"
        );
    }

    /// The restriction keys off whose turn it was, not the moved piece's colour - they differ
    /// when a Pillbug throws an enemy piece.
    #[test]
    fn set_stunned_depends_on_whose_turn_it_was() {
        let (moved_at, pillbug_at, third) = triangle(Position::new(10, 10));
        let mut board = Board::new();
        let ant = Piece::new_from(Bug::Ant, Color::Black, 1);
        board.insert(moved_at, ant, true);
        board.insert(
            pillbug_at,
            Piece::new_from(Bug::Pillbug, Color::Black, 0),
            true,
        );
        // A Queen, so there is no *White* ability piece anywhere near.
        board.insert(third, Piece::new_from(Bug::Queen, Color::White, 0), true);
        // Without a Black Queen, Black could not move the Ant anyway and the test proves nothing.
        board.insert(
            third.to(Direction::SE),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        assert!(board.queen_played(Color::Black) && board.queen_played(Color::White));

        // White threw Black's Ant: Black is about to move and Black's own Pillbug is right there.
        board.set_stunned(moved_at, ant, false, Color::White);
        assert_eq!(
            board.stunned,
            Some(ant),
            "a thrown piece may not move at all on its owner's turn"
        );

        // Black moved its own Ant: only White could be restricted, and White has nothing to do it.
        board.set_stunned(moved_at, ant, false, Color::Black);
        assert_eq!(
            board.stunned, None,
            "White has no ability piece adjacent, so nothing is restricted"
        );
    }

    /// hivegame.com/game/9ZBIYwl6Fu is the only known game with a pass after a live stun, so
    /// only this fixture makes the invariant bite.
    #[test]
    fn pass_clears_stunned() {
        use crate::canonical_hash::canonical_hash;

        let history = History::from_uhp_str(
            &std::fs::read_to_string("./test_pgns/regressions/pass_after_a_stun.uhp")
                .expect("fixture"),
        )
        .expect("valid UHP");
        let mut state = State::new(history.game_type, true);
        let mut stunned_before_the_pass = None;
        for (ply, (piece, position)) in history.moves.iter().enumerate() {
            if ply == 35 {
                stunned_before_the_pass = state.board.stunned;
                assert_eq!(piece, "pass", "ply 35 of the fixture is the pass");
            }
            state
                .play_turn_from_history(piece, position)
                .unwrap_or_else(|err| panic!("ply {ply}: {err}"));
            if ply == 35 {
                break;
            }
        }

        let stunned = stunned_before_the_pass.expect("the move before the pass left a stun");
        assert_eq!(stunned, Piece::new_from(Bug::Ant, Color::White, 1));
        assert_eq!(
            state.board.stunned, None,
            "the pass must not carry the stun forward"
        );

        // Ply 35 is odd, so White is to move once it has been played.
        let recorded = *state.hashes.last().expect("a hash per ply");
        assert_eq!(
            recorded,
            canonical_hash(&state.board, Color::White, None),
            "the pass position must hash as unrestricted"
        );
        assert_ne!(
            recorded,
            canonical_hash(&state.board, Color::White, Some(stunned)),
            "and a carried stun would have been a different position, which is the whole bug"
        );
    }

    /// The same invariant over the corpus: weak alone, but it covers games nobody hand-picked.
    #[test]
    fn no_pass_in_the_corpus_leaves_a_stun() {
        for entry in std::fs::read_dir("./test_pgns/valid/").expect("valid dir") {
            let path = entry.expect("PGN").path();
            let history = History::from_filepath(path.clone()).expect("valid PGN");
            let tournament = history
                .moves
                .iter()
                .take(2)
                .all(|(piece, _)| piece.parse::<Piece>().expect("piece").bug() != Bug::Queen);
            let mut state = State::new(history.game_type, tournament);

            let mut passes = 0;
            for (ply, (piece, position)) in history.moves.iter().enumerate() {
                state
                    .play_turn_from_history(piece, position)
                    .unwrap_or_else(|err| panic!("{}: ply {ply}: {err}", path.display()));
                if piece == "pass" {
                    passes += 1;
                    assert!(
                        state.board.stunned.is_none(),
                        "{}: ply {ply}: a pass left a stun behind",
                        path.display()
                    );
                }
            }
            // The corpus contains passes; if it stopped, this test would silently prove nothing.
            if path.to_string_lossy().contains("pass") {
                assert!(passes > 0, "{}: expected a pass", path.display());
            }
        }
    }

    /// Plies 80/84/88 are one position: the adjacent ability piece is surrounded and can throw
    /// nothing. An earlier fix asked only whether one was adjacent, and lost this draw.
    #[test]
    fn game_with_a_vacuous_restriction_is_drawn() {
        let state = replay("./test_pgns/regressions/vacuous_restriction.pgn");
        assert_eq!(state.repeating_moves, vec![80, 84, 88]);
        assert_eq!(state.game_status, GameStatus::Finished(GameResult::Draw));
    }

    /// End to end. A repetition the old rule missed entirely: the game carried on to a win after
    /// the position had already occurred three times.
    #[test]
    fn game_the_old_rule_missed_is_drawn() {
        let state = replay("./test_pgns/regressions/missed_repetition.pgn");
        assert_eq!(state.repeating_moves, vec![25, 29, 33]);
        assert_eq!(state.game_status, GameStatus::Finished(GameResult::Draw));
    }

    /// hivegame.com/game/iIPQLORgQUe9: the restricted Ant sat on a different cell at ply 35, but
    /// a stun keyed by piece type collided all three - the site drew a game that never repeated.
    #[test]
    fn game_drawn_on_a_stunned_ant_that_was_not_the_same_ant() {
        let state = replay("./test_pgns/regressions/false_draw_by_stunned_cell.pgn");
        assert!(
            state.repeating_moves.is_empty(),
            "not a repetition: {:?}",
            state.repeating_moves
        );
        assert_ne!(state.game_status, GameStatus::Finished(GameResult::Draw));
        // The whole game is there, so the draw is absent because we replayed past it.
        assert_eq!(state.turn, 46);
    }

    /// Replay a PGN as far as it goes; a threefold draw stops it before the recorded end.
    fn replay(path: &str) -> State {
        let history = History::from_filepath(path.into()).expect("valid PGN");
        let tournament = history
            .moves
            .iter()
            .take(2)
            .all(|(piece, _)| piece.parse::<Piece>().expect("piece").bug() != Bug::Queen);
        let mut state = State::new(history.game_type, tournament);
        for (piece, position) in history.moves.iter() {
            if state.play_turn_from_history(piece, position).is_err() {
                break;
            }
        }
        state
    }

    /// `set_stunned` asked only `Bug::has_move`, so a thrown Pillbug that can still throw kept
    /// no stun - a restricted position sharing an unrestricted hash, i.e. a false draw.
    #[test]
    fn set_stunned_counts_a_thrown_ability_pieces_special_action() {
        let state = replay_uhp("./test_pgns/regressions/thrown_pillbug_ability.uhp");
        assert_eq!(state.turn, 14);
        assert_eq!(state.turn_color, Color::White);

        let pillbug = Piece::new_from(Bug::Pillbug, Color::White, 0);
        let at = state
            .board
            .position_of_piece(pillbug)
            .expect("Pillbug is on the board");

        // The Pillbug was thrown, cannot walk, but can still use its ability.
        assert_eq!(state.board.last_moved, Some((pillbug, at)));
        assert!(!state.board.is_pinned(pillbug));
        assert!(!Bug::has_move(at, &state.board));
        assert!(Bug::has_available_ability(at, &state.board));

        // So the restriction is real: clearing `last_moved` gives White strictly more to do.
        let restricted = legal_targets(&state.board, Color::White);
        let mut unrestricted_board = state.board.clone();
        unrestricted_board.last_moved = None;
        let unrestricted = legal_targets(&unrestricted_board, Color::White);
        assert!(
            restricted.len() < unrestricted.len(),
            "the throw should be suppressed while the Pillbug is last_moved"
        );

        assert_eq!(state.board.stunned, Some(pillbug));
    }

    /// Every legal `piece -> target` as a flat, sorted list, so two move maps can be compared.
    fn legal_targets(board: &Board, color: Color) -> Vec<String> {
        let mut targets: Vec<String> = board
            .moves(color)
            .into_iter()
            .flat_map(|((piece, _), destinations)| {
                destinations
                    .into_iter()
                    .map(move |to| format!("{piece}->{},{}", to.q, to.r))
            })
            .collect();
        targets.sort();
        targets
    }

    /// Replay a UHP fixture the way a stored game is reconstructed.
    fn replay_uhp(path: &str) -> State {
        let input = std::fs::read_to_string(path).expect("fixture is readable");
        let history = History::from_uhp_str(&input).expect("valid UHP");
        State::new_from_history(&history).expect("legal history")
    }
    /// Before their Queen is down the opponent cannot move anything, so a restriction on them
    /// takes nothing away - marking one splits identical positions in the hash.
    #[test]
    fn set_stunned_ignores_restrictions_before_the_next_players_queen() {
        let state = replay_uhp("./test_pgns/regressions/pre_queen_vacuous_stun.uhp");
        assert_eq!(state.turn, 6);
        assert_eq!(state.turn_color, Color::White);
        assert!(!state.board.queen_played(Color::White));

        // Nothing to lose: White has no board move with the restriction and none without it.
        let restricted = legal_targets(&state.board, Color::White);
        let mut unrestricted_board = state.board.clone();
        unrestricted_board.last_moved = None;
        let unrestricted = legal_targets(&unrestricted_board, Color::White);
        assert!(restricted.is_empty());
        assert!(unrestricted.is_empty());

        assert_eq!(state.board.stunned, None);
    }
    /// Same rule, own-move branch, from a real game: the thrown piece belongs to the player who
    /// cannot move yet, so the restriction never removed an action.
    #[test]
    fn set_stunned_ignores_a_piece_thrown_before_its_owners_queen() {
        let state = replay_uhp("./test_pgns/regressions/pre_queen_thrown_piece.uhp");
        assert_eq!(state.turn, 7);
        assert_eq!(state.turn_color, Color::Black);
        assert!(!state.board.queen_played(Color::Black));

        let ladybug = Piece::new_from(Bug::Ladybug, Color::Black, 0);
        let at = state
            .board
            .position_of_piece(ladybug)
            .expect("Ladybug is on the board");
        assert_eq!(state.board.last_moved, Some((ladybug, at)));

        // Black has no board move to lose, restriction or not.
        let restricted = legal_targets(&state.board, Color::Black);
        let mut unrestricted_board = state.board.clone();
        unrestricted_board.last_moved = None;
        assert!(restricted.is_empty());
        assert!(legal_targets(&unrestricted_board, Color::Black).is_empty());

        assert_eq!(state.board.stunned, None);
    }

    /// A 694-ply game drifts the hive across the 32x32 torus; axis unwrapping must hash every
    /// ply however far it drifts.
    #[test]
    fn long_drifting_game_hashes_throughout() {
        let history =
            History::from_filepath("./test_pgns/regressions/torus_wrap.pgn".into()).expect("PGN");
        let mut state = State::new(GameType::MLP, true);
        for (ply, (piece, position)) in history.moves.iter().enumerate() {
            state
                .play_turn_from_history(piece, position)
                .unwrap_or_else(|err| panic!("ply {ply}: {err}"));
        }
        assert_eq!(state.hashes.len(), history.moves.len());
    }
    /// Replay has to apply every recorded move even though the engine now finds the repetition
    /// mid-game, otherwise the game can't be loaded at all.
    #[test]
    fn a_recorded_game_replays_whole_even_if_it_repeats() {
        let history =
            History::from_filepath("./test_pgns/regressions/missed_repetition.pgn".into())
                .expect("valid PGN");
        let state = State::new_from_history(&history).expect("a record must always reconstruct");

        assert_eq!(
            state.turn,
            history.moves.len(),
            "every recorded move should have been applied"
        );
        assert_eq!(
            state.game_status,
            GameStatus::Finished(GameResult::Winner(Color::White)),
            "the recorded result stands"
        );
        // The repetition is still noticed, it just does not overrule what was played.
        assert_eq!(state.repeating_moves, vec![25, 29, 33]);
    }
}
