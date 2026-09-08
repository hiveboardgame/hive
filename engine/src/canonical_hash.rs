//! The position hash: a pure function of pieces, side to move, and the stun - nothing else.
//!
//! Hive has no board, so identity is up to translation, rotation and reflection. Unwrap the
//! torus at each axis's guaranteed gap (a connected hive of <=28 pieces leaves one on a 32-wide
//! axis), sum a commutative key per cell under each of the 12 symmetries, keep the minimum -
//! congruent positions produce the same accumulator set, so the minimum agrees.

use crate::{
    board::{Board, BOARD_SIZE},
    color::Color,
    piece::Piece,
};

// Only the walking implementation kept for cross-checking needs these.
#[cfg(test)]
use crate::{direction::Direction, position::Position};

/// Carried over from the old hasher; the value is arbitrary, only its stability matters.
const WHITE_TO_MOVE: u64 = 0x2d358dccaa6c78a5;

/// Same-type pieces are interchangeable, so the restriction must name a cell and travel with it
/// through the symmetry transform. The stack signature's top nibble is free: 7 pieces at 4 bits.
const STUNNED_CELL: u32 = 1 << 31;

/// Unchanged from the old hasher; only which cells get hashed, and in what frame, is new.
fn wyhash(input: u64) -> u64 {
    let input = input + 0xa0761d6478bd642f_u64;
    let output: u128 = input as u128 * (input ^ 0xe7037ed1a0b428db_u64) as u128;
    ((output >> 64) ^ output) as u64
}

/// Upper bound on occupied cells: `GameType::MLP` puts 28 pieces on the board.
const MAX_CELLS: usize = 28;

#[cfg(test)]
/// [`Position::to`] applies these wrapped into the torus; here they stay true integers.
const DELTAS: [(Direction, i32, i32); 6] = [
    (Direction::NW, 0, -1),
    (Direction::SE, 0, 1),
    (Direction::NE, 1, -1),
    (Direction::SW, -1, 1),
    (Direction::W, -1, 0),
    (Direction::E, 1, 0),
];

/// The 12 hex symmetries as `[a, b, c, d]`: `q' = a*q + b*r`, `r' = c*q + d*r`. Six rotations,
/// then those mirrored.
const SYMMETRIES: [[i32; 4]; 12] = [
    [1, 0, 0, 1],
    [0, -1, 1, 1],
    [-1, -1, 1, 0],
    [-1, 0, 0, -1],
    [0, 1, -1, -1],
    [1, 1, -1, 0],
    [1, 0, -1, -1],
    [0, -1, -1, 0],
    [-1, -1, 0, 1],
    [-1, 0, 1, 1],
    [0, 1, 1, 0],
    [1, 1, 0, -1],
];

/// Occupied cells as true axial offsets plus stack signatures ([`BugStack::simple`]).
pub(crate) struct Cells {
    offsets: [(i32, i32); MAX_CELLS],
    stacks: [u32; MAX_CELLS],
    len: usize,
}

/// A connected hive of <=28 cells projects to a contiguous residue interval, so a 32-wide axis
/// always has exactly one maximal gap - cut there to unwrap. Returns the interval's left edge.
pub(crate) fn axis_origin(mask: u32) -> i32 {
    debug_assert!(mask != 0);
    let residues = || (0..BOARD_SIZE).filter(move |bit| mask & (1 << bit) != 0);
    let last = residues().next_back().expect("non-empty");

    let (mut widest, mut origin) = (-1, 0);
    let mut previous = last;
    for residue in residues() {
        let gap = (residue - previous - 1).rem_euclid(BOARD_SIZE);
        if gap > widest {
            widest = gap;
            origin = residue;
        }
        previous = residue;
    }
    origin
}

/// Reads the piece-position table rather than walking the hive: the walk was only ever there to
/// escape the torus, and [`axis_origin`] does that far more cheaply.
pub(crate) fn relative_cells(board: &Board, stunned: Option<Piece>) -> Option<Cells> {
    let mut cells = Cells {
        offsets: [(0, 0); MAX_CELLS],
        stacks: [0; MAX_CELLS],
        len: 0,
    };
    let restricted = stunned.and_then(|piece| board.position_of_piece(piece));

    // One entry per piece, so stacked cells repeat; the bitset dedupes them.
    let mut seen = [0u64; (BOARD_SIZE * BOARD_SIZE / 64) as usize];
    let (mut q_mask, mut r_mask) = (0u32, 0u32);
    let mut raw = [(0i32, 0i32); MAX_CELLS];

    for position in board.positions.iter().flatten() {
        let bit = (position.r * BOARD_SIZE + position.q) as usize;
        if seen[bit / 64] & (1 << (bit % 64)) != 0 {
            continue;
        }
        seen[bit / 64] |= 1 << (bit % 64);
        if cells.len == MAX_CELLS {
            // Only forged input gets this far; never let it silently hash like the empty board.
            debug_assert!(false, "more than {MAX_CELLS} occupied cells");
            return None;
        }
        raw[cells.len] = (position.q, position.r);
        cells.stacks[cells.len] = board.board.get(*position).simple();
        if restricted == Some(*position) {
            cells.stacks[cells.len] |= STUNNED_CELL;
        }
        cells.len += 1;
        q_mask |= 1 << position.q;
        r_mask |= 1 << position.r;
    }
    if cells.len == 0 {
        return None;
    }

    let (q_origin, r_origin) = (axis_origin(q_mask), axis_origin(r_mask));
    for (offset, (q, r)) in cells.offsets[..cells.len].iter_mut().zip(raw) {
        *offset = (
            (q - q_origin).rem_euclid(BOARD_SIZE),
            (r - r_origin).rem_euclid(BOARD_SIZE),
        );
    }
    Some(cells)
}

/// The original way of recovering offsets: walk the hive by adjacency. Kept only as an
/// independent implementation for [`forms_agree`] to check [`relative_cells`] against.
#[cfg(test)]
fn relative_cells_by_walking(board: &Board, stunned: Option<Piece>) -> Option<Cells> {
    let mut cells = Cells {
        offsets: [(0, 0); MAX_CELLS],
        stacks: [0; MAX_CELLS],
        len: 0,
    };
    let restricted = stunned.and_then(|piece| board.position_of_piece(piece));

    // Any occupied cell will do as the root. `all_taken_positions` would do a board lookup per
    // piece just to pick the top ones - more than the rest of this function costs.
    let start = *board.positions.iter().flatten().next()?;

    // Direct-addressed visit set: cheaper than hashing, and small enough to sit on the stack.
    let mut visited = [0u64; (BOARD_SIZE * BOARD_SIZE / 64) as usize];
    let mut mark = |position: Position| {
        let bit = (position.r * BOARD_SIZE + position.q) as usize;
        let word = &mut visited[bit / 64];
        let seen = *word & (1 << (bit % 64)) != 0;
        *word |= 1 << (bit % 64);
        seen
    };

    let mut queue = [(start, 0i32, 0i32); MAX_CELLS];
    let mut head = 0usize;
    let mut tail = 1usize;
    mark(start);

    let mut reached = 0usize;
    while head < tail {
        let (position, q, r) = queue[head];
        head += 1;
        let stack = board.board.get(position);
        reached += stack.size as usize;
        cells.offsets[cells.len] = (q, r);
        cells.stacks[cells.len] = stack.simple();
        if restricted == Some(position) {
            cells.stacks[cells.len] |= STUNNED_CELL;
        }
        cells.len += 1;

        for (direction, dq, dr) in DELTAS {
            let neighbor = position.to(direction);
            if !board.occupied(neighbor) || mark(neighbor) {
                continue;
            }
            if tail == MAX_CELLS {
                return None;
            }
            queue[tail] = (neighbor, q + dq, r + dr);
            tail += 1;
        }
    }

    // Connectivity check for free: every piece must sit on a cell the walk reached.
    (reached == board.played).then_some(cells)
}

/// Canonical hash of a position. `stunned` is the last-move restriction, as [`Board::stunned`].
pub fn canonical_hash(board: &Board, to_move: Color, stunned: Option<Piece>) -> u64 {
    let Some(cells) = relative_cells(board, stunned) else {
        return finish(0, to_move);
    };
    if cells.len == 0 {
        return finish(0, to_move);
    }

    let mut best = u64::MAX;
    for symmetry in SYMMETRIES {
        best = best.min(normalised(&cells, symmetry, |packed, stack| {
            wyhash(packed | stack)
        }));
    }

    finish(best, to_move)
}

/// Apply one symmetry, translate the bounding-box corner to the origin, and reduce the cells with
/// `key`. Commutative (`wrapping_add` over an unordered set), so the cells need no ordering.
fn normalised(cells: &Cells, [a, b, c, d]: [i32; 4], key: impl Fn(u64, u64) -> u64) -> u64 {
    let mut transformed = [(0i32, 0i32); MAX_CELLS];
    let (mut min_q, mut min_r) = (i32::MAX, i32::MAX);
    for (mapped, &(q, r)) in transformed[..cells.len]
        .iter_mut()
        .zip(&cells.offsets[..cells.len])
    {
        *mapped = (a * q + b * r, c * q + d * r);
        min_q = min_q.min(mapped.0);
        min_r = min_r.min(mapped.1);
    }

    let mut accumulator = 0u64;
    for (&(q, r), &stack) in transformed[..cells.len]
        .iter()
        .zip(&cells.stacks[..cells.len])
    {
        // Offsets are bounded by the piece count, so a byte each is ample.
        let packed = (((q - min_q) as u64) << 40) | (((r - min_r) as u64) << 32);
        accumulator = accumulator.wrapping_add(key(packed, stack as u64));
    }
    accumulator
}

fn finish(mut hash: u64, to_move: Color) -> u64 {
    if to_move == Color::White {
        hash ^= WHITE_TO_MOVE;
    }
    hash
}

#[cfg(test)]
/// The canonical form the hash mixes: sorted `(q, r, stack)` in the smallest orientation.
/// Exposed for validation - distinct forms on one hash is a genuine collision.
pub(crate) fn canonical_form(
    board: &Board,
    stunned: Option<Piece>,
) -> Option<Vec<(i32, i32, u32)>> {
    let cells = relative_cells(board, stunned)?;
    let mut best: Option<Vec<(i32, i32, u32)>> = None;

    for [a, b, c, d] in SYMMETRIES {
        let (mut min_q, mut min_r) = (i32::MAX, i32::MAX);
        let mut mapped = Vec::with_capacity(cells.len);
        for index in 0..cells.len {
            let (q, r) = cells.offsets[index];
            let cell = (a * q + b * r, c * q + d * r);
            min_q = min_q.min(cell.0);
            min_r = min_r.min(cell.1);
            mapped.push((cell.0, cell.1, cells.stacks[index]));
        }
        for cell in mapped.iter_mut() {
            cell.0 -= min_q;
            cell.1 -= min_r;
        }
        mapped.sort_unstable();
        if best.as_ref().is_none_or(|current| mapped < *current) {
            best = Some(mapped);
        }
    }
    best
}

/// Do the two ways of recovering offsets agree? The walking version exists only as an independent
/// implementation for the corpus cross-check to test [`relative_cells`] against.
#[cfg(test)]
pub(crate) fn forms_agree(board: &Board, stunned: Option<Piece>) -> bool {
    fn form_of(cells: &Cells) -> Option<Vec<(i32, i32, u32)>> {
        let mut best: Option<Vec<(i32, i32, u32)>> = None;
        for [a, b, c, d] in SYMMETRIES {
            let (mut min_q, mut min_r) = (i32::MAX, i32::MAX);
            let mut mapped = Vec::with_capacity(cells.len);
            for index in 0..cells.len {
                let (q, r) = cells.offsets[index];
                let cell = (a * q + b * r, c * q + d * r);
                min_q = min_q.min(cell.0);
                min_r = min_r.min(cell.1);
                mapped.push((cell.0, cell.1, cells.stacks[index]));
            }
            for cell in mapped.iter_mut() {
                cell.0 -= min_q;
                cell.1 -= min_r;
            }
            mapped.sort_unstable();
            if best.as_ref().is_none_or(|current| mapped < *current) {
                best = Some(mapped);
            }
        }
        best
    }
    let fast = relative_cells(board, stunned).as_ref().and_then(form_of);
    let walked = relative_cells_by_walking(board, stunned)
        .as_ref()
        .and_then(form_of);
    fast == walked && fast.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bug::Bug, direction::Direction, position::Position};

    /// Two predecessors converging on the same layout; only which Ant moved - and so which
    /// cell is restricted - differs.
    fn converging_predecessor(first_ant_moves: bool) -> Board {
        let centre = Position::new(10, 10);
        let mut board = Board::new();
        board.insert(centre, Piece::new_from(Bug::Pillbug, Color::Black, 0), true);
        board.insert(
            centre.to(Direction::W),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );
        board.insert(
            centre.to(Direction::SW),
            Piece::new_from(Bug::Queen, Color::White, 0),
            true,
        );

        let first = Piece::new_from(Bug::Ant, Color::White, 1);
        let second = Piece::new_from(Bug::Ant, Color::White, 2);
        if first_ant_moves {
            board.insert(centre.to(Direction::NE), first, true);
            board.insert(centre.to(Direction::SE), second, true);
        } else {
            board.insert(centre.to(Direction::E), first, true);
            board.insert(centre.to(Direction::NE), second, true);
        }

        // Placing pieces by hand is not the preceding turn.
        board.last_moved = None;
        board
    }

    /// A piece-typed stun cannot say which Ant, and a full-`Piece` stun would make the order
    /// label canonical. The rule-relevant datum is the cell.
    #[test]
    fn canonical_hash_distinguishes_the_stunned_cell() {
        let centre = Position::new(10, 10);
        let (northeast, east, southeast) = (
            centre.to(Direction::NE),
            centre.to(Direction::E),
            centre.to(Direction::SE),
        );
        let first = Piece::new_from(Bug::Ant, Color::White, 1);
        let second = Piece::new_from(Bug::Ant, Color::White, 2);

        let mut stunned_at_east = converging_predecessor(true);
        assert!(stunned_at_east.is_valid_move(Color::Black, first, northeast, east));
        stunned_at_east
            .move_piece(first, northeast, east, 5, Color::Black)
            .expect("Black throws the first Ant east");

        let mut stunned_at_southeast = converging_predecessor(false);
        assert!(stunned_at_southeast.is_valid_move(Color::Black, second, northeast, southeast));
        stunned_at_southeast
            .move_piece(second, northeast, southeast, 5, Color::Black)
            .expect("Black throws the second Ant south-east");

        // The layouts are the same position, and both restrictions are real but on different cells.
        // The layout alone - no restriction - is the same position.
        assert_eq!(
            canonical_form(&stunned_at_east, None),
            canonical_form(&stunned_at_southeast, None),
        );
        // With the restriction it is not, because the mark lands on a different cell.
        assert_ne!(
            canonical_form(&stunned_at_east, stunned_at_east.stunned),
            canonical_form(&stunned_at_southeast, stunned_at_southeast.stunned),
        );
        assert_eq!(stunned_at_east.stunned, Some(first));
        assert_eq!(stunned_at_southeast.stunned, Some(second));
        assert_eq!(stunned_at_east.position_of_piece(first), Some(east));
        assert_eq!(
            stunned_at_southeast.position_of_piece(second),
            Some(southeast)
        );

        // Which is what makes them different positions: White has different options.
        assert_ne!(
            stunned_at_east.moves(Color::White),
            stunned_at_southeast.moves(Color::White),
        );

        assert_ne!(
            canonical_hash(&stunned_at_east, Color::White, stunned_at_east.stunned),
            canonical_hash(
                &stunned_at_southeast,
                Color::White,
                stunned_at_southeast.stunned
            ),
        );
    }
    /// Rebuild a position elsewhere on the torus, rotated or mirrored - the stun `Piece` needs
    /// no transforming, which is what makes this a fair test of the mark moving with its cell.
    fn transformed(board: &Board, [a, b, c, d]: [i32; 4], (dq, dr): (i32, i32)) -> Board {
        let origin = Position::new(10, 10);
        let mut out = Board::new();
        let mut done = std::collections::HashSet::new();

        for position in board.positions.iter().flatten() {
            if !done.insert(*position) {
                continue;
            }
            let (q, r) = (position.q - origin.q, position.r - origin.r);
            let moved = Position::new(a * q + b * r + origin.q + dq, c * q + d * r + origin.r + dr);
            let stack = board.board.get(*position);
            for piece in &stack.pieces[..stack.size as usize] {
                out.insert(moved, *piece, true);
            }
        }
        out
    }

    /// A mark applied to the finished hash would survive translation and rotation without ever
    /// being attached to a cell - this is the test that says the mark is part of the position.
    #[test]
    fn the_stunned_cell_survives_every_symmetry() {
        let centre = Position::new(10, 10);
        let first = Piece::new_from(Bug::Ant, Color::White, 1);
        let mut board = converging_predecessor(true);
        board
            .move_piece(
                first,
                centre.to(Direction::NE),
                centre.to(Direction::E),
                5,
                Color::Black,
            )
            .expect("Black throws the first Ant east");
        assert_eq!(board.stunned, Some(first));

        let expected = canonical_hash(&board, Color::White, board.stunned);

        for symmetry in SYMMETRIES {
            // Translate as well, and far enough to wrap the torus, so drift is covered too.
            for shift in [(0, 0), (3, -5), (-11, 2), (14, 13)] {
                let moved = transformed(&board, symmetry, shift);
                assert_eq!(
                    canonical_hash(&moved, Color::White, moved.stunned.or(Some(first))),
                    expected,
                    "symmetry {symmetry:?} shift {shift:?} changed the hash",
                );
                // And the restriction still has to matter after the trip.
                assert_ne!(
                    canonical_hash(&moved, Color::White, None),
                    expected,
                    "symmetry {symmetry:?} shift {shift:?} lost the mark",
                );
            }
        }
    }
}
