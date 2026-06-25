use std::str::FromStr;

use super::*;
use crate::{board::Board, bug::Bug, color::Color, game_type::GameType, piece::Piece};

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
            to_hash(&canonical).unwrap(),
            expected,
            "{hop} -> {canonical}"
        );
    }
}
