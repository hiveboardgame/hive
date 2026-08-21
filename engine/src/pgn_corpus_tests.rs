//! The PGN corpora under `engine/test_pgns/`, replayed end to end. In the library, not the
//! `cli`-gated binary, so a plain `cargo test` runs them.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    canonical_hash::canonical_hash,
    color::Color,
    history::History,
    piece::Piece,
    state::State,
};

/// Every PGN in `dir`, sorted, so a failure names the same file on every machine.
fn pgns_in(dir: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("missing corpus directory {dir}: {err}"))
        .map(|entry| entry.expect("PGN").path())
        .collect();
    assert!(!files.is_empty(), "{dir} is empty, so it proves nothing");
    files.sort();
    files
}

/// Reconstruct a stored game, the way the server does when it loads one.
fn replay(file: &Path) -> Result<State, String> {
    let history = History::from_filepath(file.to_path_buf()).map_err(|err| err.to_string())?;
    State::new_from_history(&history).map_err(|err| err.to_string())
}

fn replay_expecting_success(file: &Path) -> State {
    replay(file).unwrap_or_else(|err| panic!("{} should replay: {err}", file.display()))
}

fn hashes_in(dir: &str) -> Vec<Vec<u64>> {
    pgns_in(dir)
        .iter()
        .map(|file| replay_expecting_success(file).hashes)
        .collect()
}

#[test]
fn valid_games_replay() {
    for file in pgns_in("./test_pgns/valid/") {
        replay_expecting_success(&file);
    }
}

#[test]
fn invalid_games_are_refused() {
    for file in pgns_in("./test_pgns/invalid/") {
        assert!(
            replay(&file).is_err(),
            "{} records an illegal game and must not replay",
            file.display()
        );
    }
}

#[test]
fn hash_corpus_games_replay() {
    for file in pgns_in("./test_pgns/hash/valid/") {
        replay_expecting_success(&file);
    }
}

/// These games record moves past a threefold: replay adjudicates the draw, then refuses the
/// next recorded move, so reconstructing them fails.
#[test]
fn games_that_repeat_are_refused_by_replay() {
    for file in pgns_in("./test_pgns/hash/invalid/") {
        assert!(
            replay(&file).is_err(),
            "{} repeats a position and plays on, so replay should refuse it",
            file.display()
        );
    }
}

/// A hive that comes back mirrored is the same position, so these games hash alike at every ply.
#[test]
fn mirrored_games_hash_alike() {
    let hashes = hashes_in("./test_pgns/hash/mirroring/");
    for (index, other) in hashes.iter().enumerate().skip(1) {
        assert_eq!(hashes[0], *other, "game {index} hashed differently");
    }
}

#[test]
fn rotated_games_hash_alike() {
    let hashes = hashes_in("./test_pgns/hash/rotation/");
    for (index, other) in hashes.iter().enumerate().skip(1) {
        assert_eq!(hashes[0], *other, "game {index} hashed differently");
    }
}

/// Different move orders that arrive at one position must agree on its hash, whatever they did to
/// get there - so only the final hash is comparable.
#[test]
fn the_same_position_reached_two_ways_hashes_alike() {
    let hashes = hashes_in("./test_pgns/hash/same_position/");
    for (index, other) in hashes.iter().enumerate().skip(1) {
        assert_eq!(
            hashes[0].last(),
            other.last(),
            "game {index} ended on a different hash"
        );
    }
}

/// A pass is a ply and gets a hash like any other, so the count keeps tracking the turn counter.
#[test]
fn a_game_with_a_pass_hashes_every_ply() {
    let state = replay_expecting_success(&PathBuf::from("./test_pgns/hash/short_pass.pgn"));
    assert_eq!(state.hashes.len(), state.turn);
}

/// Put the pieces back on an empty board, bottom to top, the way the HOP parser does - no move
/// history, no carried state.
fn rebuild(state: &State) -> crate::board::Board {
    let stacks = state.board.stacks();
    let mut board = crate::board::Board::new();
    for position in state.board.all_taken_positions() {
        for piece in stacks.get(position.q, position.r) {
            board.insert(position, piece, true);
        }
    }
    board
}

/// A board rebuilt from nothing but its pieces must hash to what the game recorded - the
/// property the archive index and HOP round trip stand on.
#[test]
fn the_hash_is_a_pure_function_of_the_board() {
    let mut corpus = Vec::new();
    for dir in [
        "./test_pgns/valid/",
        "./test_pgns/hash/valid/",
        "./test_pgns/hash/same_position/",
        "./test_pgns/hash/rotation/",
        "./test_pgns/hash/mirroring/",
    ] {
        corpus.extend(pgns_in(dir));
    }
    corpus.push(PathBuf::from("./test_pgns/hash/short_pass.pgn"));

    let mut validated = 0usize;
    for file in &corpus {
        let history = History::from_filepath(file.clone()).expect("valid PGN");
        let tournament = history.moves.iter().take(2).all(|(piece, _)| {
            piece.parse::<Piece>().expect("piece").bug() != crate::bug::Bug::Queen
        });
        let mut state = State::new(history.game_type, tournament);

        for (ply, (piece, position)) in history.moves.iter().enumerate() {
            if state.play_turn_from_history(piece, position).is_err() {
                break;
            }
            // The side to move follows the ply, not `turn_color`, which the engine leaves on the
            // mover when a move ends the game.
            let to_move = if ply.is_multiple_of(2) {
                Color::Black
            } else {
                Color::White
            };
            let recomputed = canonical_hash(&rebuild(&state), to_move, state.board.stunned);
            assert_eq!(
                recomputed,
                state.hashes[ply],
                "rebuilt board hashed differently at ply {ply} of {}",
                file.display()
            );
            validated += 1;
        }
    }
    println!(
        "{validated} positions rebuilt and rehashed across {} games",
        corpus.len()
    );
    assert!(validated > 0, "no positions were validated");
}
