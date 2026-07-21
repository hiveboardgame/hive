use std::{collections::HashMap, fs, str::FromStr};

use super::*;
use crate::{
    board::Board,
    bug::Bug,
    color::Color,
    game_type::GameType,
    history::History,
    piece::Piece,
    state::State,
};

fn piece(letter: char, order: usize) -> Piece {
    let color = if letter.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    let bug = Bug::from_str(&letter.to_string()).unwrap();
    Piece::new_from(bug, color, order)
}

fn adjacent(board: &Board, a: Piece, b: Piece) -> bool {
    let pa = board.position_of_piece(a).expect("a on board");
    let pb = board.position_of_piece(b).expect("b on board");
    pa.positions_around().any(|p| p == pb)
}

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

#[test]
fn parses_bent_triline() {
    let parsed = parse("QA-a,w").unwrap();
    assert_eq!(parsed.game_type, GameType::MLP);
    assert_eq!(parsed.to_move, Color::White);
    assert_eq!(parsed.board.played, 3);

    // The walk Q-A-a bends 60°, so Q touches A and A touches a, but Q does not touch a.
    let (q, a, ba) = (piece('Q', 0), piece('A', 1), piece('a', 1));
    assert!(adjacent(&parsed.board, q, a));
    assert!(adjacent(&parsed.board, a, ba));
    assert!(!adjacent(&parsed.board, q, ba));
}

#[test]
fn parses_stack() {
    let parsed = parse("QA-a2=B,b").unwrap();
    assert_eq!(parsed.board.played, 4);
    assert_eq!(parsed.to_move, Color::Black);
    let ant_cell = parsed
        .board
        .position_of_piece(piece('A', 1))
        .expect("ant on board");
    assert_eq!(parsed.board.level(ant_cell), 2);
    assert_eq!(parsed.board.top_piece(ant_cell), Some(piece('B', 1)));
}

#[test]
fn rejects_dragonfly() {
    assert_eq!(parse("Qd,w").unwrap_err(), HopError::Dragonfly);
    assert_eq!(parse("QD,w").unwrap_err(), HopError::Dragonfly);
    assert_eq!(parse("base+d,QA,w").unwrap_err(), HopError::Dragonfly);
}

#[test]
fn rejects_malformed_input() {
    assert_eq!(parse("").unwrap_err(), HopError::Empty);
    assert_eq!(parse("QA-a").unwrap_err(), HopError::FieldCount(1));
    assert_eq!(
        parse("QA-a,x").unwrap_err(),
        HopError::BadPlayer("x".to_string())
    );
    assert_eq!(
        parse("base,QM,w").unwrap_err(),
        HopError::PieceNotInGameType {
            bug: Bug::Mosquito,
            game_type: GameType::Base,
        }
    );
    assert_eq!(
        parse("QAAAA,w").unwrap_err(),
        HopError::TooManyPieces {
            color: Color::White,
            bug: Bug::Ant,
        }
    );
    assert_eq!(parse("QA-a1+(b,w").unwrap_err(), HopError::UnbalancedParens);
    assert_eq!(parse("QA-ab),w").unwrap_err(), HopError::UnbalancedParens);
}

#[test]
fn serialize_round_trips_to_same_hash() {
    for hop in [
        "QA-a,w",
        "QA-a2=B,b",
        "QA-a1+b,b",
        "A+Q+B+B3-(g+g1-g-q!),w3",
    ] {
        let mut parsed = parse(hop).unwrap();
        let canonical = from_position(&parsed.board, parsed.game_type, parsed.to_move);
        let expected = parsed.board.position_hash(parsed.to_move, None) as i64;
        assert_eq!(
            to_hash(&canonical, None).unwrap(),
            expected,
            "{hop} -> {canonical}"
        );
    }
}

#[test]
fn round_trips_empty_board() {
    for game_type in [GameType::MLP, GameType::Base, GameType::ML] {
        let hop = from_position(&Board::new(), game_type, Color::White);
        let parsed = parse(&hop).unwrap_or_else(|e| panic!("{hop}: {e}"));
        assert_eq!(parsed.board.played, 0);
        assert_eq!(parsed.game_type, game_type);
    }
    assert_eq!(parse(",w").unwrap().board.played, 0);
    assert_eq!(parse("+-!,w").unwrap_err(), HopError::NoStartBug);
}

#[test]
fn canonical_topology_distinguishes_mirror_images() {
    let clockwise = parse("QA-a,w").unwrap();
    let counter = parse("QA+a,w").unwrap();
    let clockwise_hop = from_position(&clockwise.board, clockwise.game_type, clockwise.to_move);
    let counter_hop = from_position(&counter.board, counter.game_type, counter.to_move);
    assert_ne!(clockwise_hop, counter_hop);
}

#[test]
fn rejects_malformed_orientation_suffix() {
    assert_eq!(
        parse("QA-a,w33").unwrap_err(),
        HopError::BadPlayer("w33".to_string())
    );
    assert_eq!(
        parse("QA-a,w3m3").unwrap_err(),
        HopError::BadPlayer("w3m3".to_string())
    );
}

#[test]
fn rejects_invalid_single_piece_position() {
    assert_eq!(parse("Q,w").unwrap_err(), HopError::LoneWhitePieceRequired);
    assert_eq!(parse("q,w").unwrap_err(), HopError::LoneWhitePieceRequired);
    assert_eq!(parse("q,b").unwrap_err(), HopError::LoneWhitePieceRequired);
    assert!(parse("Q,b").is_ok());
}

#[test]
fn rejects_oversized_chain_reference() {
    let hop = format!("Q{},w", "9".repeat(25));
    match parse(&hop) {
        Err(HopError::NumberTooLarge(_)) => {}
        other => panic!("expected NumberTooLarge, got {other:?}"),
    }
}

/// Per-bug-type `(movable piece count, total destination count)` for `color`. Grouping by
/// bug type rather than by exact piece (which includes an arbitrary `order` label) keeps
/// the comparison meaningful across a HOP round-trip: HOP encodes only bug letters, so
/// which specific same-type piece gets order 1 vs 2 depends on `best_topology`'s chosen
/// traversal and isn't preserved — same-type pieces are interchangeable for legality and
/// hashing (`Piece::simple` already masks out `order`), so this is the right level to compare.
fn move_profile(board: &Board, color: Color) -> HashMap<Bug, (usize, usize)> {
    let mut profile: HashMap<Bug, (usize, usize)> = HashMap::new();
    for ((piece, _pos), destinations) in board.moves(color) {
        let entry = profile.entry(piece.bug()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += destinations.len();
    }
    profile
}

#[test]
fn hop_round_trip_preserves_legal_continuation() {
    for entry in fs::read_dir("./test_pgns/valid/").expect("valid dir") {
        let path = entry.expect("PGN").path();
        let history = History::from_filepath(path.clone()).expect("valid PGN");
        let tournament = is_tournament(&history);
        let mut state = State::new(history.game_type, tournament);

        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            if ply >= 1 && state.board.stunned.is_none() {
                let to_move = state.turn_color;
                let hop = from_position(&state.board, state.game_type, to_move);
                let restored =
                    parse(&hop).unwrap_or_else(|e| panic!("{}: {hop}: {e}", path.display()));

                let expected_hash = state.hashes[ply - 1];
                let restored_hash = restored.board.clone().position_hash(to_move, None);
                assert_eq!(
                    restored_hash,
                    expected_hash,
                    "{}: ply {ply}",
                    path.display()
                );

                assert_eq!(
                    move_profile(&state.board, to_move),
                    move_profile(&restored.board, to_move),
                    "{}: ply {ply}: move profile mismatch",
                    path.display()
                );
                assert_eq!(
                    state.board.spawnable_positions(to_move).count(),
                    restored.board.spawnable_positions(to_move).count(),
                    "{}: ply {ply}: spawn count mismatch",
                    path.display()
                );
            }

            state
                .play_turn_from_history(piece, pos)
                .unwrap_or_else(|e| panic!("{}: illegal move at ply {ply}: {e}", path.display()));
        }
    }
}
