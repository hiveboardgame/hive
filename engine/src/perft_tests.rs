//! Perft against Mzinga's published counts (https://github.com/jonthysell/Mzinga/wiki/Perft):
//! the only check that our legal-move set is someone else's, not just self-consistent.
//!
//! Their conventions, mirrored below: no queen on a first turn (our tournament flag), one spawn
//! candidate per bug type, a forced pass is a move, a finished game none. Never deep enough for
//! threefold to interfere.
//!
//! PERFT_DEPTH=6 cargo test --release perft_tests:: -- --ignored --nocapture

use crate::{
    board::Board,
    bug::Bug,
    color::Color,
    game_result::GameResult,
    game_status::GameStatus,
    game_type::GameType,
    piece::Piece,
    position::Position,
    state::State,
};
use std::{collections::HashSet, str::FromStr};

const REFERENCE: [(GameType, [u64; 8]); 8] = [
    (
        GameType::Base,
        [1, 4, 96, 1440, 21600, 516240, 12219480, 181641900],
    ),
    (
        GameType::M,
        [1, 5, 150, 2610, 45414, 1252800, 34233432, 527164524],
    ),
    (
        GameType::L,
        [1, 5, 150, 2610, 45414, 1252800, 34233672, 529630188],
    ),
    (
        GameType::P,
        [1, 5, 150, 2610, 45414, 1255932, 34395984, 532753872],
    ),
    (
        GameType::ML,
        [1, 6, 216, 4320, 86400, 2725920, 85201200, 1357078404],
    ),
    (
        GameType::MP,
        [1, 6, 216, 4320, 86400, 2730888, 85492248, 1363837116],
    ),
    (
        GameType::LP,
        [1, 6, 216, 4320, 86400, 2730240, 85457136, 1366372440],
    ),
    (
        GameType::MLP,
        [1, 7, 294, 6678, 151686, 5427108, 192353904, 3151035948],
    ),
];

/// Every distinct (piece, target): moves and one spawn per bug type, deduplicated because a
/// cell reachable two ways (walk vs throw) is still one move.
fn legal_actions(state: &State) -> Vec<(Piece, Position)> {
    let color = state.turn_color;
    let mut seen: HashSet<(Piece, Position)> = HashSet::new();
    let mut actions = Vec::new();
    for ((piece, _), targets) in state.board.moves(color) {
        for target in targets {
            if seen.insert((piece, target)) {
                actions.push((piece, target));
            }
        }
    }
    let spawns: Vec<Position> = state.board.spawnable_positions(color).collect();
    for pieces in state.board.reserve(color, state.game_type).values() {
        if let Some(piece) = pieces.first().and_then(|p| Piece::from_str(p).ok()) {
            if piece.bug() == Bug::Queen && !state.queen_allowed() {
                continue;
            }
            if piece.bug() != Bug::Queen && state.queen_required_now(color) {
                continue;
            }
            for &target in &spawns {
                if seen.insert((piece, target)) {
                    actions.push((piece, target));
                }
            }
        }
    }
    actions
}

fn perft(state: &State, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    if matches!(
        state.game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    ) {
        return 0;
    }
    if state.board.is_shutout(state.turn_color, state.game_type) {
        let mut next = state.clone();
        next.play_turn_from_history("pass", "")
            .expect("a shutout pass plays");
        return perft(&next, depth - 1);
    }
    let mut total = 0;
    for (piece, target) in legal_actions(state) {
        let mut next = state.clone();
        let before = next.history.moves.len();
        next.play_turn_from_position(piece, target)
            .expect("an engine-offered action plays");
        // Auto-pass may append the opponent's forced pass; it is a move of its own.
        let consumed = (next.history.moves.len() - before) as u32;
        total += if consumed >= depth {
            1
        } else {
            perft(&next, depth - consumed)
        };
    }
    total
}

/// Must branch exactly like `perft`, or the parallel total drifts from the serial one.
fn expand(state: &State, depth: u32, levels: u32, jobs: &mut Vec<(State, u32)>, base: &mut u64) {
    if levels == 0 || depth == 0 {
        jobs.push((state.clone(), depth));
        return;
    }
    if matches!(
        state.game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    ) {
        return;
    }
    if state.board.is_shutout(state.turn_color, state.game_type) {
        let mut next = state.clone();
        next.play_turn_from_history("pass", "")
            .expect("a shutout pass plays");
        expand(&next, depth - 1, levels - 1, jobs, base);
        return;
    }
    for (piece, target) in legal_actions(state) {
        let mut next = state.clone();
        let before = next.history.moves.len();
        next.play_turn_from_position(piece, target)
            .expect("an engine-offered action plays");
        let consumed = (next.history.moves.len() - before) as u32;
        if consumed >= depth {
            *base += 1;
        } else {
            expand(&next, depth - consumed, levels - 1, jobs, base);
        }
    }
}

/// Farm the ply-2 subtrees over threads; roughly balanced at 96-294 jobs.
fn perft_parallel(state: &State, depth: u32) -> u64 {
    if depth <= 2 {
        return perft(state, depth);
    }
    let mut jobs = Vec::new();
    let mut base = 0u64;
    expand(state, depth, 2, &mut jobs, &mut base);
    let next_job = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let total = std::sync::atomic::AtomicU64::new(base);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let index = next_job.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some((job, remaining)) = jobs.get(index) else {
                    break;
                };
                total.fetch_add(perft(job, *remaining), std::sync::atomic::Ordering::Relaxed);
            });
        }
    });
    total.into_inner()
}

fn check_to_depth(max_depth: u32) {
    for (game_type, counts) in REFERENCE {
        let state = State::new(game_type, true);
        for (depth, &expected) in counts.iter().enumerate().take(max_depth as usize + 1) {
            let start = std::time::Instant::now();
            let count = perft_parallel(&state, depth as u32);
            assert_eq!(
                count, expected,
                "perft({depth}) diverged from the reference for {game_type}"
            );
            if depth >= 5 {
                println!(
                    "{game_type} perft({depth}) = {count} ({:.1}s)",
                    start.elapsed().as_secs_f32()
                );
            }
        }
    }
}

#[test]
fn perft_matches_the_reference() {
    check_to_depth(3);
}

#[test]
#[ignore = "deep sweep: PERFT_DEPTH=6 cargo test --release perft_tests:: -- --ignored --nocapture"]
fn perft_matches_the_reference_deeply() {
    let max_depth: u32 = std::env::var("PERFT_DEPTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5);
    check_to_depth(max_depth.min(7));
}

/// Same tree as `perft`, walked with `Board::make`/`unmake` instead of cloning `State`s.
fn perft_make(
    board: &mut Board,
    game_type: GameType,
    color: Color,
    turn: usize,
    depth: u32,
) -> u64 {
    if depth == 0 {
        return 1;
    }
    if board.game_result() != GameResult::Unknown {
        return 0;
    }
    let mut actions = Vec::new();
    board.for_each_move(color, |piece, from, to| {
        actions.push((piece, Some(from), to))
    });
    board.for_each_placement(color, game_type, turn > 1, |piece, to| {
        actions.push((piece, None, to))
    });
    if actions.is_empty() {
        let saved = (board.last_moved, board.last_move, board.stunned);
        board.last_moved = None;
        board.last_move = (None, None);
        board.stunned = None;
        let total = perft_make(
            board,
            game_type,
            color.opposite_color(),
            turn + 1,
            depth - 1,
        );
        (board.last_moved, board.last_move, board.stunned) = saved;
        return total;
    }
    let mut total = 0;
    for (piece, from, to) in actions {
        let unmake = board.make(piece, from, to, color);
        total += perft_make(
            board,
            game_type,
            color.opposite_color(),
            turn + 1,
            depth - 1,
        );
        board.unmake(unmake);
    }
    total
}

#[test]
fn make_unmake_perft_matches_the_reference() {
    let max_depth: u32 = std::env::var("PERFT_DEPTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4);
    for (game_type, counts) in REFERENCE {
        for (depth, &expected) in counts.iter().enumerate().take(max_depth as usize + 1) {
            let mut board = Board::new();
            let count = perft_make(&mut board, game_type, Color::White, 0, depth as u32);
            assert_eq!(
                count, expected,
                "make/unmake perft({depth}) diverged from the reference for {game_type}"
            );
            assert_eq!(
                board,
                Board::new(),
                "perft must leave the board as it found it"
            );
        }
    }
}
