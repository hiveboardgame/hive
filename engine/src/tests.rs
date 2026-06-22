use super::*;
use hive_lib::{Board, Bug, Piece};
use std::fs;

/// Tournament rule (as `State::play_and_print`): on unless either side opens with a Queen.
fn is_tournament(history: &History) -> bool {
    for idx in 0..2 {
        if let Some((piece_str, _)) = history.moves.get(idx) {
            let piece: Piece = piece_str.parse().expect("piece");
            if piece.bug() == Bug::Queen {
                return false;
            }
        }
    }
    true
}

/// The valid PGN corpus used by the rehash/position-hash equivalence tests.
fn hash_corpus_files() -> Vec<PathBuf> {
    let dirs = [
        "./test_pgns/valid/",
        "./test_pgns/hash/valid/",
        "./test_pgns/hash/same_position/",
        "./test_pgns/hash/rotation/",
        "./test_pgns/hash/mirroring/",
    ];
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        for entry in fs::read_dir(dir).unwrap_or_else(|_| panic!("missing dir {dir}")) {
            files.push(entry.expect("PGN").path());
        }
    }
    files.push(PathBuf::from("./test_pgns/hash/short_pass.pgn"));
    files
}

/// Rebuild a `Board` by spawning each piece bottom-to-top, as the HOP parser does (no carried hash).
fn rebuild_board(src: &Board) -> Board {
    let stacks = src.stacks();
    let mut board = Board::new();
    for position in src.all_taken_positions() {
        for piece in stacks.get(position.q, position.r) {
            board.insert(position, piece, true);
        }
    }
    board
}

/// Assert `Board::rehash` equals the incremental hash at every ply; returns `(validated, anchor_changes)`.
fn full_rehash_matches_incremental(file: PathBuf) -> (usize, usize) {
    let history = History::from_filepath(file.clone()).expect("valid PGN");
    let mut state = State::new(history.game_type, is_tournament(&history));
    let mut validated = 0;
    let mut anchor_changed = 0;
    let (mut prev_smallest, mut prev_eigen) = (None, None);

    for (ply, (piece, pos)) in history.moves.iter().enumerate() {
        state
            .play_turn_from_history(piece, pos)
            .unwrap_or_else(|e| panic!("illegal move at ply {ply} of {}: {e}", file.display()));

        if state.board.smallest != prev_smallest || state.board.eigen_direction != prev_eigen {
            anchor_changed += 1;
        }
        prev_smallest = state.board.smallest;
        prev_eigen = state.board.eigen_direction;

        // Passes and the first piece skip the spiral rehash we're validating.
        if piece == "pass" || state.board.played < 2 {
            continue;
        }

        let incremental = *state.hashes.last().expect("hash recorded for this ply");
        // `hash_move` ran with `turn == ply` for this position, so re-hash with the same parity.
        let recomputed = state.board.clone().rehash(ply);
        assert_eq!(
            incremental,
            recomputed,
            "full rehash != incremental at ply {ply} of {}",
            file.display()
        );
        validated += 1;
    }

    (validated, anchor_changed)
}

#[test]
fn test_full_rehash_matches_incremental() {
    let files = hash_corpus_files();
    let (mut total_validated, mut total_anchor_changed) = (0, 0);
    for file in &files {
        let (validated, anchor_changed) = full_rehash_matches_incremental(file.clone());
        total_validated += validated;
        total_anchor_changed += anchor_changed;
    }

    println!(
        "validated {total_validated} positions across {} games; full-rehash branch fired {total_anchor_changed} times",
        files.len()
    );
    // Confirm we exercised the rehash and that the production `else` branch is reachable.
    assert!(total_validated > 0, "no positions were validated");
    assert!(
        total_anchor_changed > 0,
        "the full-rehash (`else`) branch never fired across the corpus"
    );
}

/// Assert `Board::position_hash` on a rebuilt board equals the played hash at every ply.
fn position_hash_matches_played(file: PathBuf) -> usize {
    let history = History::from_filepath(file.clone()).expect("valid PGN");
    let mut state = State::new(history.game_type, is_tournament(&history));
    let mut validated = 0;

    for (ply, (piece, pos)) in history.moves.iter().enumerate() {
        state
            .play_turn_from_history(piece, pos)
            .unwrap_or_else(|e| panic!("illegal move at ply {ply} of {}: {e}", file.display()));

        // A pass leaves `stunned` stale and the first piece is a separate path; both skipped here.
        if piece == "pass" || state.board.played < 2 {
            continue;
        }

        let rebuilt = rebuild_board(&state.board);
        assert_eq!(
            rebuilt.clone().compute_smallest().map(|(_, p)| p),
            state.board.smallest.map(|(_, p)| p),
            "anchor mismatch at ply {ply} of {}",
            file.display()
        );

        // Derive side-to-move from the ply; `turn_color` is stale on the game-ending ply.
        let to_move = if ply % 2 == 0 {
            Color::Black
        } else {
            Color::White
        };
        let recomputed = rebuilt.clone().position_hash(to_move, state.board.stunned);
        assert_eq!(
            recomputed,
            *state.hashes.last().expect("hash recorded for this ply"),
            "position_hash != played hash at ply {ply} of {}",
            file.display()
        );
        validated += 1;
    }

    validated
}

#[test]
fn test_position_hash_matches_played_hash() {
    let files = hash_corpus_files();
    let mut total = 0;
    for file in &files {
        total += position_hash_matches_played(file.clone());
    }
    println!(
        "position_hash matched the played hash for {total} positions across {} games",
        files.len()
    );
    assert!(total > 0, "no positions were validated");
}

#[test]
fn test_play_games_from_valid_files() {
    for entry in fs::read_dir("./test_pgns/valid/").expect("Should be valid directory") {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        assert!(play_game_from_file(entry).is_ok());
    }
}

#[test]
fn test_play_games_from_invalid_files() {
    for entry in fs::read_dir("./test_pgns/invalid/").expect("Should be valid directory") {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        assert!(play_game_from_file(entry).is_err());
    }
}

#[test]
fn test_hash_from_valid_files() {
    for entry in fs::read_dir("./test_pgns/hash/valid/").expect("Should be valid directory") {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        assert!(play_game_from_file(entry).is_ok());
    }
}

#[test]
fn test_hash_from_invalid_files() {
    for entry in fs::read_dir("./test_pgns/hash/invalid/").expect("Should be valid directory") {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        assert!(play_game_from_file(entry).is_err());
    }
}

#[test]
fn test_hash_mirroring_from_files() {
    let mut hashes = Vec::new();
    for entry in fs::read_dir("./test_pgns/hash/mirroring/").expect("Should be valid directory") {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        match play_game_from_file(entry) {
            Err(e) => panic!("{}", e.to_string()),
            Ok(state) => hashes.push(state.hashes),
        };
    }
    assert_eq!(hashes[0], hashes[1]);
    assert_eq!(hashes[0], hashes[2]);
}

#[test]
fn test_hash_same_position_from_files() {
    let mut hashes = Vec::new();
    for entry in fs::read_dir("./test_pgns/hash/same_position/").expect("Should be valid directory")
    {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        match play_game_from_file(entry) {
            Err(e) => panic!("{}", e.to_string()),
            Ok(state) => hashes.push(state.hashes),
        };
    }
    assert_eq!(hashes[0].last(), hashes[1].last());
}

#[test]
fn test_hash_rotation_from_files() {
    let mut hashes = Vec::new();
    for entry in fs::read_dir("./test_pgns/hash/rotation/").expect("Should be valid directory") {
        let entry = entry.expect("PGN").path();
        println!("{}", entry.display());
        match play_game_from_file(entry) {
            Err(e) => panic!("{}", e.to_string()),
            Ok(state) => hashes.push(state.hashes),
        };
    }
    assert_eq!(hashes[0], hashes[1]);
}

#[test]
fn test_hash_pass_from_file() {
    let file = PathBuf::from("./test_pgns/hash/short_pass.pgn");
    println!("{}", file.display());
    match play_game_from_file(file) {
        Err(e) => panic!("{}", e.to_string()),
        Ok(state) => {
            assert_eq!(state.hashes.len(), state.turn);
        }
    };
}
