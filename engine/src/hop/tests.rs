use std::{collections::HashMap, fs, str::FromStr};

use super::*;
use crate::{
    board::{Board, BOARD_SIZE},
    bug::Bug,
    color::Color,
    direction::Direction,
    game_result::GameResult,
    game_status::GameStatus,
    game_type::GameType,
    history::History,
    piece::Piece,
    position::Position,
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
fn a_loaded_position_still_owes_its_queen_by_the_fourth_turn() {
    let parsed = parse("A+G+S-a-g-s,w").unwrap();
    let mut state =
        State::new_from_position(parsed.board, parsed.game_type, parsed.to_move).unwrap();
    let spawn = state
        .board
        .spawnable_positions(Color::White)
        .next()
        .expect("White has a spawnable cell");
    assert!(
        state
            .clone()
            .play_turn_from_position(piece('G', 2), spawn)
            .is_err(),
        "White has placed three and owes the queen"
    );
    state
        .play_turn_from_position(piece('Q', 0), spawn)
        .expect("the queen itself is always allowed");
}

/// A long snake of a hive walked off the torus edge on load and rendered in three pieces.
/// Loading must land the whole hive on one side of the seam, centred on the spawn position.
#[test]
fn load_does_not_wrap_around_the_torus() {
    let parsed = parse("A+BMBPSLGGSAAGqapgbgblmgsasaQ,w").unwrap();
    let (mut min_q, mut max_q) = (BOARD_SIZE, -1);
    let (mut min_r, mut max_r) = (BOARD_SIZE, -1);
    for at in parsed.board.all_taken_positions() {
        min_q = min_q.min(at.q);
        max_q = max_q.max(at.q);
        min_r = min_r.min(at.r);
        max_r = max_r.max(at.r);
    }
    // A wrapped hive occupies both edge residues of an axis; one clear of both edges cannot
    // be wrapped.
    assert!(
        min_q > 0 && max_q < BOARD_SIZE - 1,
        "hive wraps the q axis: q spans {min_q}..={max_q}"
    );
    assert!(
        min_r > 0 && max_r < BOARD_SIZE - 1,
        "hive wraps the r axis: r spans {min_r}..={max_r}"
    );
    let centre = Position::initial_spawn_position();
    assert_eq!(min_q + (max_q - min_q) / 2, centre.q, "not centred on q");
    assert_eq!(min_r + (max_r - min_r) / 2, centre.r, "not centred on r");
}

#[test]
fn serialize_round_trips_to_same_hash() {
    for hop in [
        "QA-a,w",
        "QA-a2=B,b",
        "QA-a1+b,b",
        "A+Q+B+B3-(g+g1-g-q!),w3",
    ] {
        let parsed = parse(hop).unwrap();
        let canonical = from_position(&parsed.board, parsed.game_type, parsed.to_move);
        let expected =
            crate::canonical_hash::canonical_hash(&parsed.board, parsed.to_move, None) as i64;
        assert_eq!(
            to_hash(&canonical).unwrap(),
            expected,
            "{hop} -> {canonical}"
        );
    }
}

#[test]
fn round_trips_empty_board() {
    for game_type in [GameType::MLP, GameType::Base] {
        let hop = from_position(&Board::new(), game_type, Color::White);
        let parsed = parse(&hop).unwrap_or_else(|e| panic!("{hop}: {e}"));
        assert_eq!(parsed.board.played, 0);
        assert_eq!(parsed.game_type, game_type);
    }
    assert_eq!(parse(",w").unwrap().board.played, 0);
    assert_eq!(parse("+-,w").unwrap_err(), HopError::NoStartBug);
    assert_eq!(parse("+-!,w").unwrap_err(), HopError::MisplacedMark);
}

#[test]
fn canonical_topology_distinguishes_mirror_images() {
    let clockwise = parse("QA-a,w").unwrap();
    let counter = parse("QA+a,w").unwrap();
    let clockwise_hop = from_position(&clockwise.board, clockwise.game_type, clockwise.to_move);
    let counter_hop = from_position(&counter.board, counter.game_type, counter.to_move);
    assert_ne!(clockwise_hop, counter_hop);
}

/// Dropping it would silently rewrite the HOP a user pasted.
#[test]
fn the_orientation_suffix_survives_a_round_trip() {
    let plain = parse("QA-a,w").unwrap();
    let canonical = from_position(&plain.board, plain.game_type, plain.to_move);
    assert_eq!(canonical, "A+a1-Q,w");

    for (suffix, expected) in [
        ("", "A+a1-Q,w"),
        ("0", "A+a1-Q,w0"),
        ("3", "A+a1-Q,w3"),
        ("5m", "A+a1-Q,w5m"),
        ("m", "A+a1-Q,wm"),
        ("M", "A+a1-Q,wm"),
    ] {
        let parsed = parse(&format!("QA-a,w{suffix}")).unwrap();
        let emitted = from_position_oriented(
            &parsed.board,
            parsed.game_type,
            parsed.to_move,
            parsed.orientation,
        );
        assert_eq!(emitted, expected, "w{suffix}");
        assert_eq!(parse(&emitted).unwrap().orientation, parsed.orientation);
    }
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
    // Either colour may have opened (the `,b` sandbox lets Black), but never the side to move.
    assert_eq!(parse("Q,w").unwrap_err(), HopError::LonePieceOwnerToMove);
    assert_eq!(parse("q,b").unwrap_err(), HopError::LonePieceOwnerToMove);
    assert!(parse("Q,b").is_ok());
    assert!(parse("q,w").is_ok());
}

#[test]
fn rejects_oversized_chain_reference() {
    let hop = format!("Q{},w", "9".repeat(25));
    match parse(&hop) {
        Err(HopError::NumberTooLarge(_)) => {}
        other => panic!("expected NumberTooLarge, got {other:?}"),
    }
}

/// Per-bug-type move counts: HOP does not preserve order labels, so bug type is the finest
/// granularity that survives a round-trip.
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
        // Only the two types people actually play: the rest add corpus time without coverage.
        if !matches!(history.game_type, GameType::Base | GameType::MLP) {
            continue;
        }
        let tournament = is_tournament(&history);
        let mut state = State::new(history.game_type, tournament);

        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            if ply >= 1 {
                let to_move = state.turn_color;
                let hop = from_position(&state.board, state.game_type, to_move);
                let restored =
                    parse(&hop).unwrap_or_else(|e| panic!("{}: {hop}: {e}", path.display()));

                let expected_hash = state.hashes[ply - 1];
                // The stun comes from the round trip itself: `!` carries it and `parse` recovers it.
                let restored_hash = crate::canonical_hash::canonical_hash(
                    &restored.board,
                    to_move,
                    restored.board.stunned,
                );
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

/// Sorted hashes of every one-move continuation - piece identities do not survive a HOP
/// round-trip, but the set of resulting positions must.
fn continuation_hashes(state: &State, color: Color) -> Vec<u64> {
    let mut hashes = Vec::new();
    for ((piece, _), destinations) in state.board.moves(color) {
        for destination in destinations {
            let mut next = state.clone();
            if next.play_turn_from_position(piece, destination).is_ok() {
                hashes.extend(next.hashes.last().copied());
            }
        }
    }
    hashes.sort_unstable();
    hashes
}

/// A state rebuilt from a bare position must play on like the game it came from. Sampled:
/// expanding every continuation at every ply is too slow for the corpus.
#[test]
fn state_from_position_plays_on_like_the_real_game() {
    for entry in fs::read_dir("./test_pgns/valid/").expect("valid dir") {
        let path = entry.expect("PGN").path();
        let history = History::from_filepath(path.clone()).expect("valid PGN");
        if !matches!(history.game_type, GameType::Base | GameType::MLP) {
            continue;
        }
        let mut state = State::new(history.game_type, is_tournament(&history));

        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            let sampled = ply >= 1 && (ply <= 4 || ply.is_multiple_of(16));
            if sampled {
                let to_move = state.turn_color;
                let hop = from_position(&state.board, state.game_type, to_move);
                let restored = parse(&hop).expect("HOP round trips");
                let base =
                    State::new_from_position(restored.board, restored.game_type, restored.to_move)
                        .unwrap();

                assert_eq!(
                    base.turn_color,
                    to_move,
                    "{}: ply {ply}: wrong side to move",
                    path.display()
                );
                assert_eq!(
                    base.turn.is_multiple_of(2),
                    state.turn.is_multiple_of(2),
                    "{}: ply {ply}: turn parity must match the real game",
                    path.display()
                );
                assert_eq!(
                    continuation_hashes(&base, to_move),
                    continuation_hashes(&state, to_move),
                    "{}: ply {ply}: continuations hash differently",
                    path.display()
                );
            }

            state
                .play_turn_from_history(piece, pos)
                .unwrap_or_else(|e| panic!("{}: illegal move at ply {ply}: {e}", path.display()));
        }
    }
}

/// Before this was enforced, `Q1=A1=B1=G1=S1=L1=M1=P,w` panicked inside `BugStack::push_piece`
/// - from a function whose signature promises a `Result`.
#[test]
fn rejects_stacking_a_bug_that_cannot_climb() {
    assert_eq!(
        parse("Q1=A,w").unwrap_err(),
        HopError::CannotClimb { bug: Bug::Ant }
    );
    assert_eq!(
        parse("Q1=A1=B1=G1=S1=L1=M1=P,w").unwrap_err(),
        HopError::CannotClimb { bug: Bug::Ant }
    );
    for letter in ['a', 'g', 's', 'l', 'p', 'q'] {
        let hop = format!("Q1={letter},w");
        assert!(
            matches!(parse(&hop), Err(HopError::CannotClimb { .. })),
            "{hop} should have been rejected"
        );
    }
}

/// The tallest stack Hive can build: a ground piece under every climber in the game - two Beetles
/// and one Mosquito a side. Seven is also exactly what `BugStack` holds.
#[test]
fn accepts_the_tallest_legal_stack() {
    // Both Queens are on the board, so every climber has a side that could have moved it.
    let parsed = parse("Qq1=B1=b1=B1=b1=M1=m,w").expect("seven is legal");
    let cell = parsed
        .board
        .position_of_piece(piece('Q', 0))
        .expect("queen on board");
    assert_eq!(parsed.board.level(cell), 7, "seven high");
    // Eight pieces in total: the seven in the stack plus Black's Queen beside it.
    assert_eq!(parsed.board.played, 8);
}

/// A chain that doubles back onto a cell it already filled is stacking by the back door; the same
/// rule applies there as to an explicit `N=X`.
#[test]
fn rejects_a_chain_that_walks_onto_an_occupied_cell() {
    // Six turns of a hexagon bring the walk back to where it started.
    assert!(matches!(
        parse("QA+G+S+A+G+S,w"),
        Err(HopError::CannotClimb { .. })
    ));
}

#[test]
fn a_state_cannot_be_built_from_an_unreachable_position() {
    let state_from = |hop: &str| {
        let parsed = parse(hop).expect("HOP itself stays lax");
        State::new_from_position(parsed.board, parsed.game_type, parsed.to_move)
    };
    // A fourth placement without the Queen: the deadline came and went.
    assert!(state_from("A+G+S+B-a-g-s-b,w").is_err());
    // Turns alternate, so a queenless side cannot be two behind.
    assert!(state_from("A+G+S,b").is_err());
    assert!(state_from("A+G+S-a,w").is_err());

    assert!(state_from("A+G+S-a-g-s,w").is_ok(), "three each is level");
    assert!(state_from("A,b").is_ok(), "White opening is one ahead");
    assert!(
        state_from("A+G+S+Q-a-g-s,b").is_ok(),
        "the fourth may be the Queen"
    );
}

/// Nothing gets on top of the hive without moving, and nothing moves before its owner's Queen is
/// down. A Pillbug throw cannot put a piece up there either - it only throws to empty cells.
#[test]
fn requires_a_queen_before_anything_climbs() {
    assert_eq!(
        parse("Qa1=b,w").unwrap_err(),
        HopError::QueenRequiredToMove {
            color: Color::Black
        }
    );
    // Black's Queen on the board makes the same climb legal.
    assert!(parse("Qq1=b,w").is_ok());
}

/// Find a four-ply "out and back" cycle that returns to the position it started from.
fn find_return_cycle(
    state: &State,
) -> Option<(Piece, Position, Position, Piece, Position, Position)> {
    let mover = state.turn_color;
    let waiter = mover.opposite_color();
    for ((piece_a, origin_a), destinations_a) in state.board.moves(mover) {
        for destination_a in destinations_a {
            let mut after_a = state.clone();
            if after_a
                .play_turn_from_position(piece_a, destination_a)
                .is_err()
            {
                continue;
            }
            for ((piece_b, origin_b), destinations_b) in after_a.board.moves(waiter) {
                for destination_b in destinations_b {
                    let mut cycle = after_a.clone();
                    if cycle
                        .play_turn_from_position(piece_b, destination_b)
                        .is_err()
                        || cycle.play_turn_from_position(piece_a, origin_a).is_err()
                        || cycle.play_turn_from_position(piece_b, origin_b).is_err()
                    {
                        continue;
                    }
                    if cycle.hashes.last().copied()
                        == Some(crate::canonical_hash::canonical_hash(
                            &state.board,
                            mover,
                            state.board.stunned,
                        ))
                    {
                        return Some((
                            piece_a,
                            destination_a,
                            origin_a,
                            piece_b,
                            destination_b,
                            origin_b,
                        ));
                    }
                }
            }
        }
    }
    None
}

/// A HOP-loaded position has occurred once already: two returns make a threefold, but only if
/// the root is counted.
#[test]
fn threefold_counts_the_hop_root() {
    let history = History::from_filepath("./test_pgns/regressions/missed_repetition.pgn".into())
        .expect("PGN");
    let mut state = State::new(GameType::MLP, true);
    for (piece, pos) in history.moves.iter().take(20) {
        state.play_turn_from_history(piece, pos).expect("legal");
    }

    let hop = from_position(&state.board, state.game_type, state.turn_color);
    let restored = parse(&hop).expect("round trips");
    let mut rooted =
        State::new_from_position(restored.board, restored.game_type, restored.to_move).unwrap();
    assert!(rooted.hashes.is_empty(), "history and hashes stay aligned");

    let (piece_a, dest_a, origin_a, piece_b, dest_b, origin_b) =
        find_return_cycle(&rooted).expect("a shuffle exists in this position");

    for cycle in 1..=2 {
        for (piece, destination) in [
            (piece_a, dest_a),
            (piece_b, dest_b),
            (piece_a, origin_a),
            (piece_b, origin_b),
        ] {
            rooted
                .play_turn_from_position(piece, destination)
                .unwrap_or_else(|e| panic!("cycle {cycle}: {e}"));
        }
    }

    let root = rooted.hashes.last().copied().expect("moves were played");
    assert_eq!(
        rooted.hashes_count.get(&root),
        Some(&3),
        "root, plus two returns to it"
    );
    assert_eq!(rooted.game_status, GameStatus::Finished(GameResult::Draw));
}

/// The final inventory decides, not the spelling: letter order, explicit counts and removals
/// from `ultimate` all name the same set. Unequal inventories are not Hive, and are refused.
#[test]
fn game_types_are_judged_by_the_inventory_they_produce() {
    assert_eq!(parse("base,QA-a,b").unwrap().game_type, GameType::Base);
    assert_eq!(parse("ultimate,QA-a,b").unwrap().game_type, GameType::MLP);
    // An omitted game type is the two-field form, which is how `ultimate` is written.
    assert_eq!(parse("QA-a,b").unwrap().game_type, GameType::MLP);
    for (spelling, game_type) in [
        ("base+Mm", GameType::M),
        ("base+Ll", GameType::L),
        ("base+Pp", GameType::P),
        ("base+MLml", GameType::ML),
        ("base+LPlp", GameType::LP),
        ("base+MPmp", GameType::MP),
        ("base+mM", GameType::M),
        ("base+MmLl", GameType::ML),
        ("base+MLPmlp", GameType::MLP),
        ("base+PplLmM", GameType::MLP),
        ("base+1M1m", GameType::M),
        ("ultimate-Pp", GameType::ML),
        ("ultimate-1P1p", GameType::ML),
        ("ultimate-Mm-Ll", GameType::P),
        ("base+Mm-Mm", GameType::Base),
        ("base+1M1L1P1m1l1p", GameType::MLP),
    ] {
        assert_eq!(
            parse(&format!("{spelling},QA-a,b")).unwrap().game_type,
            game_type,
            "{spelling}"
        );
    }

    for unsupported in [
        "base+m",
        "base+M",
        "base+ml",
        "base+MmL",
        "base+MMmm",
        "base+2M2m",
        "base-1A",
        "base+3a",
        "base+Qq",
        "base+",
        "ultimate-1P",
        "ultimate+Mm",
    ] {
        let hop = format!("{unsupported},QA-a,b");
        assert!(
            matches!(parse(&hop), Err(HopError::UnsupportedGameType(_))),
            "{hop} should have been refused, got {:?}",
            parse(&hop)
        );
    }
}

/// A game of any supported type can be serialized and loaded back: an existing
/// partial-expansion game's HOP used to be refused by the very application that wrote it.
#[test]
fn every_game_type_round_trips_through_hop() {
    for game_type in [
        GameType::Base,
        GameType::M,
        GameType::L,
        GameType::P,
        GameType::ML,
        GameType::LP,
        GameType::MP,
        GameType::MLP,
    ] {
        let mut board = Board::new();
        let centre = Position::new(16, 16);
        board.insert(centre, Piece::new_from(Bug::Queen, Color::White, 0), true);
        board.insert(
            centre.to(Direction::E),
            Piece::new_from(Bug::Queen, Color::Black, 0),
            true,
        );

        let hop = from_position(&board, game_type, Color::White);
        let restored = parse(&hop).unwrap_or_else(|e| panic!("{game_type:?}: {hop}: {e}"));
        assert_eq!(restored.game_type, game_type, "{hop}");
        assert_eq!(restored.board.played, 2, "{hop}");
    }
}

/// An empty board with Black to move starts at turn 1, so the old `turn == 1` transition in
/// `next_turn` was skipped forever and the analysis stayed `NotStarted` while being played.
#[test]
fn playing_from_an_artificial_position_starts_the_game() {
    let restored = parse(",b").expect("an empty board with Black to move parses");
    let mut state =
        State::new_from_position(restored.board, restored.game_type, restored.to_move).unwrap();
    assert_eq!(state.turn, 1);
    assert_eq!(state.game_status, GameStatus::NotStarted);

    state
        .play_turn_from_history("bS1", "")
        .expect("Black opens on the empty board");
    assert_eq!(state.game_status, GameStatus::InProgress);
}

/// Copy HOP -> Load HOP must round trip for every state the sandbox can reach.
#[test]
fn sandbox_states_reload() {
    let side_to_move = |state: &State| {
        if state.turn.is_multiple_of(2) {
            Color::White
        } else {
            Color::Black
        }
    };

    // The queen falling due on the fourth placement.
    let parsed = parse("A+G+S-a-g-s,w").unwrap();
    let mut state =
        State::new_from_position(parsed.board, parsed.game_type, parsed.to_move).unwrap();
    let spawn = state
        .board
        .spawnable_positions(Color::White)
        .next()
        .unwrap();
    state.play_turn_from_position(piece('Q', 0), spawn).unwrap();
    let hop = from_position(&state.board, state.game_type, side_to_move(&state));
    parse(&hop).unwrap_or_else(|e| panic!("copied sandbox HOP must reload: {e} ({hop})"));

    // Black opening a bare board.
    let parsed = parse(",b").unwrap();
    let mut state =
        State::new_from_position(parsed.board, parsed.game_type, parsed.to_move).unwrap();
    let spawn = state
        .board
        .spawnable_positions(Color::Black)
        .next()
        .unwrap();
    state.play_turn_from_position(piece('q', 0), spawn).unwrap();
    let hop = from_position(&state.board, state.game_type, side_to_move(&state));
    parse(&hop).unwrap_or_else(|e| panic!("copied Black-opening HOP must reload: {e} ({hop})"));
}

/// Spellings other HOP implementations emit (https://pepeke.app/help/hop) must load and mean
/// the same position and game type as our canonical form.
#[test]
fn literal_inventories_name_the_same_game_types_as_the_aliases() {
    for spelling in [
        "Q3A2B3G2Sq3a2b3g2s",   // as the spec writes base
        "3A2B3G2SQ3a2b3g2sq",   // any token order
        "Qq3A3a2B2b3G3g2S2s",   // colours interleaved
        "1Q3A2B3G2S1q3a2b3g2s", // an explicit count of one
        "QAAA2B3G2Sqaaa2b3g2s", // repeats accumulate
        "Q3A2B3G2S+q3a2b3g2s",  // a literal list plus modifiers
    ] {
        let hop = format!("{spelling},QA-a,w");
        let parsed = parse(&hop).unwrap_or_else(|e| panic!("{spelling}: {e}"));
        assert_eq!(parsed.game_type, GameType::Base, "{spelling}");
    }

    assert_eq!(
        parse("QMLP3A2B3G2Sqmlp3a2b3g2s,QA-a,w").unwrap().game_type,
        GameType::MLP
    );
    // Unequal sides are Pepeke's vocabulary, not Hive, whatever the spelling.
    assert!(parse("QM3A2B3G2Sq3a2b3g2s,QA-a,w").is_err());
}

#[test]
fn spec_spellings_from_other_implementations_load() {
    // Aliases and modifier forms that all mean ultimate (Base+MLP).
    for spelling in [
        "ultimate,QA-a,w",
        "base+MLPmlp,QA-a,w",
        "base+1M1m1L1l1P1p,QA-a,w",
        "base+Mm+Ll+Pp,QA-a,w",
        "base+lMpmLP,QA-a,w",
        "ultimate+Ss-Ss,QA-a,w",
    ] {
        let parsed = parse(spelling).unwrap_or_else(|e| panic!("{spelling}: {e}"));
        assert_eq!(parsed.game_type, GameType::MLP, "{spelling}");
        assert_eq!(
            to_hash(spelling).unwrap(),
            to_hash("QA-a,w").unwrap(),
            "{spelling}"
        );
    }
    // Subtraction forms of the smaller game types.
    assert_eq!(
        parse("ultimate-MmLlPp,QA-a,w").unwrap().game_type,
        GameType::Base
    );
    assert_eq!(parse("ultimate-Pp,QA-a,w").unwrap().game_type, GameType::ML);
    // Orientation suffixes are presentation only: accepted, same position.
    for player in ["w", "w0", "w3", "w5m", "wm"] {
        let hop = format!("QA-a,{player}");
        assert_eq!(to_hash(&hop).unwrap(), to_hash("QA-a,w").unwrap(), "{hop}");
    }
    // Our engine plays exactly the eight symmetric game types: at most one M/L/P per side,
    // both sides equal. Everything else in HOP's wider vocabulary is refused, cleanly.
    assert!(matches!(parse("QD,w"), Err(HopError::Dragonfly)));
    for unsupported in [
        "base+MMmm,QA-a,w",  // two Mosquitoes a side
        "base+2M2m,QA-a,w",  // the same, spelled with counts
        "base+Mm+Mm,QA-a,w", // the same, spelled across modifier groups
        "base+M,QA-a,w",     // uneven: only White gains a Mosquito
        "ultimate-P,QA-a,w", // uneven: only White loses the Pillbug
        "base+4A4a,QA-a,w",  // seven Ants a side
    ] {
        assert!(parse(unsupported).is_err(), "{unsupported} must be refused");
    }
}

/// `!` names the bug before it, so a misplaced one silently changed the position: `Q!A-a,w`
/// stuns wQ, `!QA-a,w` stuns nothing.
#[test]
fn rejects_malformed_topology_controls() {
    assert_eq!(parse("!Q,b").unwrap_err(), HopError::MisplacedMark);
    assert_eq!(parse("Q(),b").unwrap_err(), HopError::EmptyScope);
    assert_eq!(parse("Q1+,b").unwrap_err(), HopError::EmptyBranch);
    assert!(parse("Q!,b").is_ok());
    assert!(parse("QA-a,w").is_ok());
}
