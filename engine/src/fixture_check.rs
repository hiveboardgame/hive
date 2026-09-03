//! Repetition fixtures, checked against the engine and against [`position_identity`] - an
//! oracle owing nothing to the hash: same board up to the 12 hex symmetries and translation,
//! same legal moves. The stun is deliberately not part of that key; a restriction that matters
//! already shows in the move set, and keying on a vacuous one would split identical positions.

use crate::{
    board::Board,
    color::Color,
    game_result::GameResult,
    game_status::GameStatus,
    history::History,
    piece::Piece,
    state::State,
};

/// Read a fixture, echoing a hivegame.com analysis link - libtest shows it only on failure,
/// handing a human a board to open the moment something breaks.
fn fixture(path: &str) -> String {
    let uhp = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("fixture {path} is unreadable: {err}"))
        .trim()
        .to_string();
    ECHOED.with(|seen| {
        if seen.borrow_mut().insert(path.to_string()) {
            println!("--- fixture {path}");
            println!("    {uhp}");
            println!(
                "    https://hivegame.com/analysis?uhp={}",
                percent_encode(&uhp)
            );
        }
    });
    uhp
}

thread_local! {
    /// Per test - libtest gives each test its own thread - so every failing test prints the
    /// fixtures it touched, once each, however many times it replays them.
    static ECHOED: std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

/// RFC 3986 unreserved set; everything else escaped. `;`, `+`, `\` and `/` all occur in UHP
/// and all need it.
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn replay_uhp(path: &str) -> State {
    let history = History::from_uhp_str(&fixture(path)).expect("valid UHP");
    State::new_from_history(&history).expect("legal history")
}

fn replay_uhp_prefix(path: &str, plies: usize) -> State {
    let history = History::from_uhp_str(&fixture(path)).expect("valid UHP");
    let tournament =
        history.moves.iter().take(2).all(|(piece, _)| {
            piece.parse::<Piece>().expect("piece").bug() != crate::bug::Bug::Queen
        });
    let mut state = State::new(history.game_type, tournament);
    for (piece, position) in history.moves.iter().take(plies) {
        state
            .play_turn_from_history(piece, position)
            .expect("prefix must be legal");
    }
    state
}

fn report(name: &str, state: &State) {
    println!(
        "{name}: plies={} status={:?} repeating_moves={:?}",
        state.hashes.len(),
        state.game_status,
        state.repeating_moves
    );
}

/// Positive control for translation invariance: the same position at plies 2/4/6 on three
/// different sets of hexes. Fails loudly if tightening the hash breaks legitimate pooling.
#[test]
fn fixture_shuffle_check() {
    let path = "./test_pgns/regressions/shuffle_check.uhp";
    let state = replay_uhp(path);
    report("shuffle check", &state);
    assert_eq!(state.repeating_moves, vec![1, 3, 5], "draws at plies 2/4/6");

    let (w2, b2, h2) = queens_and_hash(path, 2);
    let (w4, b4, h4) = queens_and_hash(path, 4);
    let (w6, b6, h6) = queens_and_hash(path, 6);
    println!("  ply2 wQ{w2:?} bQ{b2:?}\n  ply4 wQ{w4:?} bQ{b4:?}\n  ply6 wQ{w6:?} bQ{b6:?}");

    // Three different places on the board...
    assert_ne!(w2, w4);
    assert_ne!(w4, w6);
    assert_ne!(b2, b4);
    assert_ne!(b4, b6);
    // ...the same relative arrangement, so the same orientation throughout...
    let offset = |w: (i32, i32), b: (i32, i32)| (b.0 - w.0, b.1 - w.1);
    assert_eq!(offset(w2, b2), offset(w4, b4));
    assert_eq!(offset(w2, b2), offset(w6, b6));
    // ...and therefore one hash.
    assert_eq!(h2, h4, "translation must not change the hash");
    assert_eq!(h2, h6, "translation must not change the hash");

    let census = repetition_census(path, 6);
    assert_eq!(
        census.most, 3,
        "a genuine threefold: same position at plies 2, 4, 6"
    );
    assert!(
        census.repeated.contains(&vec![2, 4, 6]),
        "{:?}",
        census.repeated
    );
}

/// Same control as [`fixture_shuffle_check`] for rotation: each occurrence is the previous
/// one turned 120 degrees.
#[test]
fn fixture_rotation_check() {
    let path = "./test_pgns/regressions/rotation_check.uhp";
    let state = replay_uhp(path);
    report("rotation check", &state);
    assert_eq!(state.repeating_moves, vec![1, 3, 5], "draws at plies 2/4/6");

    let (w2, b2, h2) = queens_and_hash(path, 2);
    let (w4, b4, h4) = queens_and_hash(path, 4);
    let (w6, b6, h6) = queens_and_hash(path, 6);
    let offset = |w: (i32, i32), b: (i32, i32)| (b.0 - w.0, b.1 - w.1);
    let (o2, o4, o6) = (offset(w2, b2), offset(w4, b4), offset(w6, b6));
    println!("  offsets at plies 2/4/6: {o2:?} {o4:?} {o6:?}");

    // Three genuinely different orientations - this is the rotation, not a translation.
    assert_eq!(o2, (1, 0), "bQ east of wQ");
    assert_eq!(o4, (-1, 1), "turned 120 degrees: bQ south-west");
    assert_eq!(o6, (0, -1), "turned again: bQ north-west");
    assert_ne!(w2, w4);
    assert_ne!(w4, w6);
    // ...and it is still one position.
    assert_eq!(h2, h4, "rotation must not change the hash");
    assert_eq!(h2, h6, "rotation must not change the hash");

    let census = repetition_census(path, 6);
    assert_eq!(
        census.most, 3,
        "a genuine threefold: same position at plies 2, 4, 6"
    );
    assert!(
        census.repeated.contains(&vec![2, 4, 6]),
        "{:?}",
        census.repeated
    );
}

/// Two different move sequences reaching the same positions must produce identical hash
/// sequences, not just draw at the same plies.
#[test]
fn fixture_shuffle_and_rotation_agree() {
    let shuffle = replay_uhp("./test_pgns/regressions/shuffle_check.uhp");
    let rotation = replay_uhp("./test_pgns/regressions/rotation_check.uhp");
    println!("  shuffle:  {:x?}", shuffle.hashes);
    println!("  rotation: {:x?}", rotation.hashes);
    assert_eq!(
        shuffle.hashes, rotation.hashes,
        "the same position reached two ways must hash the same at every ply"
    );
    assert_eq!(shuffle.repeating_moves, rotation.repeating_moves);
}

/// The third symmetry class, reflection: ply 9 mirrors plies 7/11 across the NE line, one
/// position under the mirror-pooling ruling. Also exercises repetition across passes.
#[test]
fn fixture_symmetry_check() {
    let path = "./test_pgns/regressions/symmetry_check.uhp";

    // The draw lands on the move that completes the threefold, not on the pass after it.
    let at_11 = replay_uhp_prefix(path, 11);
    report("symmetry check (11 plies)", &at_11);
    assert_eq!(at_11.game_status, GameStatus::Finished(GameResult::Draw));
    assert_eq!(
        at_11.repeating_moves,
        vec![6, 8, 10],
        "the repeated positions are plies 7/9/11"
    );

    let layout = |plies: usize| {
        let state = replay_uhp_prefix(path, plies);
        let origin = state
            .board
            .position_of_piece(Piece::new_from(crate::bug::Bug::Queen, Color::White, 0))
            .expect("wQ anchors the frame");
        let mut cells = std::collections::BTreeMap::new();
        for pos in Board::all_positions() {
            let stack = state.board.board.get(pos);
            if !stack.is_empty() {
                cells.insert((pos.q - origin.q, pos.r - origin.r), stack.simple());
            }
        }
        (cells, *state.hashes.last().expect("a hash per ply"))
    };
    let (l7, h7) = layout(7);
    let (l9, h9) = layout(9);
    let (l11, h11) = layout(11);
    println!("  ply 7  {l7:?}\n  ply 9  {l9:?}\n  ply 11 {l11:?}");

    // Plies 7 and 11 are literally the same; ply 9 is not.
    assert_eq!(l7, l11);
    assert_ne!(l7, l9, "ply 9 puts wA2 on the other side of the line");

    // ...but it is the reflection of it. In cube coordinates the reflection fixing the NE
    // axis - the line wQ, bQ, wA1 sit on - is (x, y, z) -> (-z, -y, -x).
    let reflect = |(q, r): (i32, i32)| {
        let (x, y, z) = (q, -q - r, r);
        let (x, _y, z) = (-z, -y, -x);
        (x, z)
    };
    assert_eq!(reflect((1, -1)), (1, -1), "the NE axis is fixed");
    let mirrored: std::collections::BTreeMap<(i32, i32), u32> =
        l7.iter().map(|(c, s)| (reflect(*c), *s)).collect();
    assert_eq!(mirrored, l9, "ply 9 is exactly the mirror image of ply 7");

    let census = repetition_census(path, 11);
    assert_eq!(census.most, 3, "a genuine threefold at plies 7, 9, 11");
    assert!(
        census.repeated.contains(&vec![7, 9, 11]),
        "{:?}",
        census.repeated
    );

    // And therefore one position, one hash, three occurrences.
    assert_eq!(h7, h9, "a position and its reflection share a hash");
    assert_eq!(h7, h11);

    // The game is over AND the player to move is shut out, which is what used to make the
    // auto-pass fire past the end of the game. See `the_deciding_move_does_not_auto_pass`.
    assert_eq!(at_11.turn_color, Color::Black);
    assert!(
        at_11.board.is_shutout(at_11.turn_color, at_11.game_type),
        "shut out, so an unguarded auto-pass would fire after the game already finished"
    );
}

/// The loser of a threefold is usually shut out, so the auto-pass used to fire after the game
/// was decided and record a phantom ply. Driven through the live path, the only one that passes.
#[test]
fn the_deciding_move_does_not_auto_pass() {
    use crate::position::Position;

    let path = "./test_pgns/regressions/symmetry_check.uhp";
    let mut state = replay_uhp_prefix(path, 10);
    assert_eq!(state.turn, 10);
    assert_eq!(state.hashes.len(), 10);

    let piece: Piece = "wA2".parse().expect("piece");
    let target = Position::from_string("-wQ", &state.board).expect("position");
    state
        .play_turn_from_position(piece, target)
        .expect("the deciding move is legal");

    assert_eq!(
        state.game_status,
        GameStatus::Finished(GameResult::Draw),
        "the move completes the threefold"
    );
    assert!(
        state.board.is_shutout(state.turn_color, state.game_type),
        "and leaves the player to move shut out, so the guard is what stops the pass"
    );

    assert_eq!(state.turn, 11, "eleven plies were played");
    assert_eq!(state.history.moves.len(), 11, "and eleven recorded");
    assert_eq!(state.hashes.len(), 11, "one hash per ply, no phantom");
    assert_ne!(
        state.history.moves.last().map(|(piece, _)| piece.as_str()),
        Some("pass"),
        "the game ended on a move, not on a pass played after it"
    );
    assert_eq!(state.repeating_moves, vec![6, 8, 10]);
}

/// Where both Queens sit after `plies`, plus the hash at that point.
fn queens_and_hash(path: &str, plies: usize) -> ((i32, i32), (i32, i32), u64) {
    let state = replay_uhp_prefix(path, plies);
    let at = |c| {
        state
            .board
            .position_of_piece(Piece::new_from(crate::bug::Bug::Queen, c, 0))
            .map(|p| (p.q, p.r))
            .expect("Queen is on the board")
    };
    (
        at(Color::White),
        at(Color::Black),
        *state.hashes.last().expect("a hash per ply"),
    )
}

#[test]
fn fixture_02_false_draw_own_throws() {
    let path = "./test_pgns/regressions/false_draw_own_throws.uhp";
    let state = replay_uhp(path);
    report("02 false_draw_own_throws", &state);
    assert_eq!(state.hashes.len(), 39, "all 39 plies replay");
    assert!(state.repeating_moves.is_empty(), "the false draw is gone");
    assert_ne!(
        state.hashes[30], state.hashes[34],
        "ply 31 vs 35 must split"
    );
    assert_eq!(state.hashes[30], state.hashes[38], "plies 31 and 39 pool");
    let bp = Piece::new_from(crate::bug::Bug::Pillbug, Color::Black, 0);
    let at_31 = replay_uhp_prefix(path, 31);
    assert_eq!(at_31.board.stunned, Some(bp), "the thrown bP is stunned");

    // What actually makes these different positions, owing nothing to the hash: the layout is
    // the same at all three plies, but Black has strictly fewer moves right after the throw.
    let (at_35, at_39) = (replay_uhp_prefix(path, 35), replay_uhp_prefix(path, 39));
    assert_eq!(layout_of(&at_31), layout_of(&at_35), "same layout");
    assert_eq!(layout_of(&at_31), layout_of(&at_39), "same layout");
    assert_eq!(at_31.turn_color, Color::Black);
    assert_eq!(at_35.turn_color, Color::Black);
    assert_eq!(at_39.turn_color, Color::Black);

    let m31 = moves_of(&at_31, Color::Black);
    let m35 = moves_of(&at_35, Color::Black);
    let m39 = moves_of(&at_39, Color::Black);
    assert_eq!(m31, m39, "plies 31 and 39 really are the same position");
    assert_ne!(
        m31, m35,
        "ply 35 is not, so pooling it would be a false draw"
    );

    // And the difference is exactly the thrown Pillbug's own throws - the actions `moves`
    // suppresses while it is last_moved, and which `set_stunned` used to miss entirely.
    let gained: Vec<&String> = m35.iter().filter(|m| !m31.contains(m)).collect();
    let lost: Vec<&String> = m31.iter().filter(|m| !m35.contains(m)).collect();
    println!(
        "  ply 35 has {} moves, plies 31/39 have {}",
        m35.len(),
        m31.len()
    );
    println!("  only at ply 35: {gained:?}");
    assert!(
        lost.is_empty(),
        "the restriction only ever removes: {lost:?}"
    );
    assert_eq!(
        gained,
        vec!["wG1@16,15->14,16", "wG2@15,17->14,16", "wG3@14,17->14,16",],
        "the three throws the thrown bP would have granted Black"
    );

    // So nothing occurs three times, and the draw the old code called was false.
    let census = repetition_census(path, 39);
    assert_eq!(census.most, 2, "no position occurs three times");
    assert!(
        census.repeated.contains(&vec![31, 39]),
        "31 and 39 pool with each other and nothing else: {:?}",
        census.repeated
    );
}

/// How often each position recurs over the first `plies` plies, judged by
/// [`position_identity`] and owing nothing to the hash.
struct Census {
    most: usize,
    repeated: Vec<Vec<usize>>,
}

fn repetition_census(path: &str, plies: usize) -> Census {
    let mut groups: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for ply in 1..=plies {
        groups
            .entry(position_identity(&replay_uhp_prefix(path, ply)))
            .or_default()
            .push(ply);
    }
    let mut repeated: Vec<Vec<usize>> = groups.values().filter(|v| v.len() > 1).cloned().collect();
    repeated.sort();
    let most = groups.values().map(Vec::len).max().unwrap_or(0);
    println!(
        "  census: {} distinct positions in {plies} plies; repeated {repeated:?}; most = {most}x",
        groups.len()
    );
    Census { most, repeated }
}

/// The 12 hex symmetries as `[a, b, c, d]`, meaning `q' = a*q + b*r`, `r' = c*q + d*r`.
/// Spelled out here rather than imported, so the oracle owes nothing to the hash it checks.
const HEX_SYMMETRIES: [[i32; 4]; 12] = [
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

/// Identity up to the 12 hex symmetries and translation: raw coordinates would split congruent
/// positions and hide a genuine threefold. Layout and moves transform together, into one frame.
fn position_identity(state: &State) -> String {
    identity(state, true)
}

/// The layout half of [`position_identity`]: a sound pre-filter (identical positions are
/// congruent layouts, not vice versa), so layout groups are candidates needing refinement.
fn layout_identity(state: &State) -> String {
    identity(state, false)
}

/// A legal move under a symmetry transform: `(from, to, piece)` in axial coordinates.
type OrientedMove = ((i32, i32), (i32, i32), u8);

fn identity(state: &State, with_moves: bool) -> String {
    let mut cells: Vec<((i32, i32), u32)> = Vec::new();
    for pos in Board::all_positions() {
        let stack = state.board.board.get(pos);
        if !stack.is_empty() {
            cells.push(((pos.q, pos.r), stack.simple()));
        }
    }
    let moves: Vec<OrientedMove> = if with_moves {
        state
            .board
            .moves(state.turn_color)
            .into_iter()
            .flat_map(|((piece, from), targets)| {
                targets
                    .into_iter()
                    .map(move |to| ((from.q, from.r), (to.q, to.r), piece.simple()))
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut best: Option<String> = None;
    for [a, b, c, d] in HEX_SYMMETRIES {
        let map = |(q, r): (i32, i32)| (a * q + b * r, c * q + d * r);
        let turned: Vec<((i32, i32), u32)> = cells.iter().map(|(p, s)| (map(*p), *s)).collect();
        // Translation is pinned by the layout's bounding box, and the moves are shifted by the
        // same offset - targets may sit outside that box, which is fine.
        let min_q = turned
            .iter()
            .map(|((q, _), _)| *q)
            .min()
            .expect("non-empty");
        let min_r = turned
            .iter()
            .map(|((_, r), _)| *r)
            .min()
            .expect("non-empty");
        let shift = |(q, r): (i32, i32)| (q - min_q, r - min_r);

        let mut layout: Vec<((i32, i32), u32)> =
            turned.iter().map(|(p, s)| (shift(*p), *s)).collect();
        layout.sort_unstable();
        let mut available: Vec<OrientedMove> = moves
            .iter()
            .map(|(from, to, piece)| (shift(map(*from)), shift(map(*to)), *piece))
            .collect();
        available.sort_unstable();

        let rendering = format!("{layout:?}|{available:?}");
        if best.as_ref().is_none_or(|current| rendering < *current) {
            best = Some(rendering);
        }
    }
    format!("{:?}|{}", state.turn_color, best.expect("a hive has cells"))
}

#[test]
fn fixture_04_mirror_pooling() {
    let path = "./test_pgns/regressions/mirror_pooling.uhp";
    let state = replay_uhp(path);
    report("04 mirror_pooling", &state);
    // The bug: a draw is called although the true position occurred only twice.
    assert_eq!(
        state.repeating_moves,
        vec![3, 7, 11],
        "mirror pooling fires"
    );
    let bq = Piece::new_from(crate::bug::Bug::Queen, Color::Black, 0);
    let cell = |plies| {
        replay_uhp_prefix(path, plies)
            .board
            .position_of_piece(bq)
            .unwrap()
    };
    let (p4, p8, p12) = (cell(4), cell(8), cell(12));
    println!("  bQ after plies 4/8/12: {p4} {p8} {p12}");
    assert_eq!(p4, p12, "plies 4 and 12 are the same position");
    assert_ne!(p8, p4, "ply 8 is the MIRROR, not the same position");

    // Under the ruling that a mirror image is the same position, this draw is correct: the
    // oracle pools ply 8 with 4 and 12 exactly as the hash does.
    let census = repetition_census(path, 12);
    assert_eq!(
        census.most, 3,
        "three occurrences once mirrors count as one position"
    );
    assert!(
        census.repeated.contains(&vec![4, 8, 12]),
        "{:?}",
        census.repeated
    );
}

/// Occupied cells and their stack signatures, at absolute coordinates.
fn layout_of(state: &State) -> Vec<String> {
    let mut cells: Vec<String> = Vec::new();
    for pos in Board::all_positions() {
        let stack = state.board.board.get(pos);
        if !stack.is_empty() {
            cells.push(format!("{}@{},{}", stack.simple(), pos.q, pos.r));
        }
    }
    cells.sort();
    cells
}

/// Every legal `piece@from->to` for `color`, source included - dropping the source is exactly
/// what blinds `corpus_tests::detected_repetitions_are_real` to this class of bug.
fn moves_of(state: &State, color: Color) -> Vec<String> {
    let mut out: Vec<String> = state
        .board
        .moves(color)
        .into_iter()
        .flat_map(|((piece, from), targets)| {
            targets
                .into_iter()
                .map(move |to| format!("{piece}@{},{}->{},{}", from.q, from.r, to.q, to.r))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn fixture_03_location_blind_stun() {
    let path = "./test_pgns/regressions/location_blind_stun.uhp";
    let state = replay_uhp(path);
    report("03 location_blind_stun", &state);
    assert_eq!(state.hashes.len(), 25, "all 25 plies replay");
    assert!(state.repeating_moves.is_empty(), "the false draw is gone");

    let occ1 = replay_uhp_prefix(path, 17);
    let occ2 = replay_uhp_prefix(path, 21);
    let occ3 = replay_uhp_prefix(path, 25);
    let p_cell = crate::position::Position::new(17, 15);
    let q_cell = crate::position::Position::new(16, 17);
    let stun_cell = |s: &State| s.board.position_of_piece(s.board.stunned.unwrap()).unwrap();
    assert_eq!(stun_cell(&occ1), p_cell);
    assert_eq!(stun_cell(&occ2), q_cell);
    assert_eq!(stun_cell(&occ3), p_cell);
    println!(
        "  stun cells P/Q/P, hashes {:x} {:x} {:x}",
        occ1.hashes.last().unwrap(),
        occ2.hashes.last().unwrap(),
        occ3.hashes.last().unwrap()
    );
    assert_eq!(occ1.hashes.last(), occ3.hashes.last(), "P-stuns pool");
    assert_ne!(occ1.hashes.last(), occ2.hashes.last(), "Q-stun splits off");

    // What is TRUE of the position, owing nothing to the code under test.
    assert_eq!(layout_of(&occ1), layout_of(&occ2), "same layout");
    assert_eq!(layout_of(&occ1), layout_of(&occ3), "same layout");
    assert_eq!(occ1.turn_color, Color::Black);
    assert_eq!(occ2.turn_color, Color::Black);
    assert_eq!(occ3.turn_color, Color::Black);

    let (m1, m2, m3) = (
        moves_of(&occ1, Color::Black),
        moves_of(&occ2, Color::Black),
        moves_of(&occ3, Color::Black),
    );
    assert_eq!(m1, m3, "occ1 and occ3 really are the same position");
    assert_ne!(
        m1, m2,
        "occ2 must differ, or the pooled draw would have been correct"
    );

    // ...and therefore, over the whole game, nothing occurs three times.
    let census = repetition_census(path, 25);
    assert_eq!(
        census.most, 2,
        "no position occurs three times, so any draw here is false"
    );
}

#[test]
fn fixture_05_edge_axis_mirror_stun() {
    let path = "./test_pgns/regressions/edge_axis_mirror_stun.uhp";
    let state = replay_uhp(path);
    report("05 edge_axis_mirror_stun", &state);
    // A genuine threefold: the draw must fire on time, at ply 27.
    assert_eq!(state.repeating_moves, vec![18, 22, 26], "draws on time");
    assert_eq!(state.hashes[18], state.hashes[22]);
    assert_eq!(state.hashes[18], state.hashes[26]);

    let stun_cell = |plies| {
        let s = replay_uhp_prefix(path, plies);
        let c = s.board.position_of_piece(s.board.stunned.unwrap()).unwrap();
        (c.q, c.r)
    };
    println!(
        "  stun cells at 19/23/27: {:?} {:?} {:?}",
        stun_cell(19),
        stun_cell(23),
        stun_cell(27)
    );
    assert_eq!(stun_cell(19), (17, 15));
    assert_eq!(stun_cell(23), (17, 16), "mirror-paired cell");
    assert_eq!(stun_cell(27), (17, 15));

    // The claim this fixture exists to make: the draw at ply 27 is CORRECT, i.e. one position
    // really did occur three times. Checked against the independent oracle - same board up to
    // the 12 hex symmetries, same side to move, same available moves for that side. Ply 23 is
    // the reflection of 19 and 27, so pooling it is the ruling working, not a collision.
    let a = position_identity(&replay_uhp_prefix(path, 19));
    let b = position_identity(&replay_uhp_prefix(path, 23));
    let c = position_identity(&replay_uhp_prefix(path, 27));
    assert_eq!(a, b, "ply 23 is the same position as ply 19, mirrored");
    assert_eq!(a, c, "ply 27 is the same position as ply 19");

    let census = repetition_census(path, 27);
    assert_eq!(census.most, 3, "a genuine threefold, so the draw is right");
    assert!(
        census.repeated.contains(&vec![19, 23, 27]),
        "and it is the one at plies 19/23/27: {:?}",
        census.repeated
    );
}

/// The layout of witness 05 really is symmetric about the edge-type axis through wP, which is
/// what made it the red team's counterexample.
#[test]
fn fixture_05_layout_is_edge_axis_symmetric() {
    let state = replay_uhp_prefix("./test_pgns/regressions/edge_axis_mirror_stun.uhp", 19);
    let mut layout = std::collections::BTreeMap::new();
    for pos in Board::all_positions() {
        let bs = state.board.board.get(pos);
        if !bs.is_empty() {
            layout.insert((pos.q, pos.r), bs.simple());
        }
    }
    let sigma = |c: (i32, i32)| {
        let (q, r) = (c.0 - 16, c.1 - 16);
        (q + 16, -q - r + 16)
    };
    assert!(
        layout
            .iter()
            .all(|(c, s)| layout.get(&sigma(*c)) == Some(s)),
        "layout must be symmetric under the edge-type reflection"
    );
    assert_eq!(sigma((17, 15)), (17, 16), "sigma pairs the two stun cells");
}

// ---- corpus validation ---------------------------------------------------------------------

fn read_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            }
            '"' => quoted = true,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => {}
            c => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Every engine-drawn threefold in the corpus, confirmed by the oracle at the claimed plies -
/// the symmetry-aware check `corpus_tests::detected_repetitions_are_real` cannot make.
#[test]
#[ignore = "needs MLP_GAMES_CSV"]
fn corpus_every_detected_draw_is_a_real_threefold() {
    let path = std::env::var("MLP_GAMES_CSV").expect("set MLP_GAMES_CSV");
    let text = std::fs::read_to_string(&path).expect("readable corpus");
    let games = read_csv(&text);
    let limit: usize = std::env::var("GAME_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);

    let (mut drawn, mut confirmed, mut plies_matched) = (0usize, 0usize, 0usize);
    let mut false_draws: Vec<String> = Vec::new();
    let mut ply_mismatch: Vec<String> = Vec::new();

    for row in games[1..].iter().take(limit) {
        let Ok(history) = History::new_from_str(&row[3]) else {
            continue;
        };
        let tournament = history
            .moves
            .iter()
            .take(2)
            .all(|(p, _)| p.parse::<Piece>().map(|p| p.bug()) != Ok(crate::bug::Bug::Queen));

        let mut state = State::new(crate::game_type::GameType::MLP, tournament);
        for (piece, pos) in history.moves.iter() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
        }
        if state.repeating_moves.len() < 3 {
            continue;
        }
        drawn += 1;

        // Census this game with the independent oracle.
        let mut replay = State::new(crate::game_type::GameType::MLP, tournament);
        let mut groups: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            if replay.play_turn_from_history(piece, pos).is_err() {
                break;
            }
            groups
                .entry(position_identity(&replay))
                .or_default()
                .push(ply);
        }
        let most = groups.values().map(Vec::len).max().unwrap_or(0);
        if most >= 3 {
            confirmed += 1;
            // The engine's claimed plies should themselves be one of the >=3 groups.
            let claimed: Vec<usize> = state.repeating_moves.clone();
            if groups.values().any(|g| {
                g.len() >= 3 && claimed.iter().all(|c| g.contains(c)) && claimed.len() >= 3
            }) {
                plies_matched += 1;
            } else {
                ply_mismatch.push(format!("{} claimed {claimed:?}", row[0]));
            }
        } else {
            false_draws.push(format!("{} most={most}", row[0]));
        }
    }

    println!("games the engine drew by repetition: {drawn}");
    println!("  oracle confirms a real threefold:  {confirmed}");
    println!("  and at the very plies claimed:     {plies_matched}");
    println!("  FALSE DRAWS:                       {}", false_draws.len());
    for g in false_draws.iter().take(20) {
        println!("    {g}");
    }
    for g in ply_mismatch.iter().take(20) {
        println!("    ply mismatch: {g}");
    }
    assert!(false_draws.is_empty(), "false draws: {false_draws:?}");
}

/// The engine must draw exactly the games where a position occurs three times. Layout pre-filter
/// first: move generation at ~5M plies would be needlessly slow.
#[test]
#[ignore = "needs MLP_GAMES_CSV"]
fn corpus_engine_and_oracle_agree_on_every_game() {
    let path = std::env::var("MLP_GAMES_CSV").expect("set MLP_GAMES_CSV");
    let text = std::fs::read_to_string(&path).expect("readable corpus");
    let games = read_csv(&text);
    let limit: usize = std::env::var("GAME_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);

    let (mut checked, mut agreed, mut drawn, mut refined) = (0usize, 0usize, 0usize, 0usize);
    let mut engine_drew_oracle_did_not: Vec<String> = Vec::new();
    let mut oracle_found_engine_missed: Vec<String> = Vec::new();
    let mut near_miss: Vec<String> = Vec::new();

    for row in games[1..].iter().take(limit) {
        let Ok(history) = History::new_from_str(&row[3]) else {
            continue;
        };
        let tournament = history
            .moves
            .iter()
            .take(2)
            .all(|(p, _)| p.parse::<Piece>().map(|p| p.bug()) != Ok(crate::bug::Bug::Queen));
        let new = || State::new(crate::game_type::GameType::MLP, tournament);

        // Pass 1: replay once, keyed on layout only.
        let mut state = new();
        let mut by_layout: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (played, (piece, pos)) in history.moves.iter().enumerate() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
            by_layout
                .entry(layout_identity(&state))
                .or_default()
                .push(played);
            if matches!(state.game_status, GameStatus::Finished(_)) {
                break;
            }
        }
        checked += 1;
        let engine_drew = state.repeating_moves.len() >= 3;
        if engine_drew {
            drawn += 1;
        }

        // Pass 2: refine only the layout groups that could possibly hold a threefold.
        let candidates: Vec<Vec<usize>> = by_layout
            .into_values()
            .filter(|plies| plies.len() >= 3)
            .collect();
        let mut oracle_threefold = false;
        if !candidates.is_empty() {
            refined += 1;
            let wanted: std::collections::HashSet<usize> =
                candidates.iter().flatten().copied().collect();
            let mut replay = new();
            let mut by_position: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for (ply, (piece, pos)) in history.moves.iter().enumerate() {
                if replay.play_turn_from_history(piece, pos).is_err() {
                    break;
                }
                if wanted.contains(&ply) {
                    let count = by_position.entry(position_identity(&replay)).or_default();
                    *count += 1;
                    if *count >= 3 {
                        oracle_threefold = true;
                    }
                }
                if matches!(replay.game_status, GameStatus::Finished(_)) {
                    break;
                }
            }
        }

        // Layout and side to move are equal by construction, so only `last_moved` can make
        // the move sets differ.
        if !candidates.is_empty() && !oracle_threefold {
            // Report plies 1-based, the way the site numbers moves.
            let mut groups: Vec<Vec<usize>> = candidates
                .iter()
                .map(|g| g.iter().map(|p| p + 1).collect())
                .collect();
            groups.sort();
            near_miss.push(format!("{:<14} plies {groups:?}", row[0]));
        }

        if engine_drew == oracle_threefold {
            agreed += 1;
        } else if engine_drew {
            engine_drew_oracle_did_not.push(row[0].clone());
        } else {
            oracle_found_engine_missed.push(row[0].clone());
        }
    }

    println!("games replayed:                    {checked}");
    println!("  engine called a threefold:       {drawn}");
    println!("  needed the expensive refinement: {refined}");
    println!("  engine and oracle agree:         {agreed}");
    println!(
        "  layout repeated 3x but NOT a threefold: {}",
        near_miss.len()
    );
    for g in near_miss.iter() {
        println!("    {g}");
    }
    println!(
        "  FALSE DRAWS (engine yes, oracle no): {}",
        engine_drew_oracle_did_not.len()
    );
    for g in engine_drew_oracle_did_not.iter().take(20) {
        println!("    {g}");
    }
    println!(
        "  MISSED DRAWS (oracle yes, engine no): {}",
        oracle_found_engine_missed.len()
    );
    for g in oracle_found_engine_missed.iter().take(20) {
        println!("    {g}");
    }
    assert!(
        engine_drew_oracle_did_not.is_empty(),
        "false draws: {engine_drew_oracle_did_not:?}"
    );
    assert!(
        oracle_found_engine_missed.is_empty(),
        "missed draws: {oracle_found_engine_missed:?}"
    );
}
