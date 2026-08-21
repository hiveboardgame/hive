//! Corpus checks for threefold repetition detection, run against a production export.
//!
//! Ignored by default; they need CSVs that are not in the repo. Run with:
//!
//! ```text
//! MLP_GAMES_CSV=/path/to/mlp_games.csv MLP_RESULTS_CSV=/path/to/mlp_results.csv \
//!   cargo test --release -p hive --lib corpus_tests -- --ignored --nocapture
//! ```
//!
//! Exports produced by:
//!
//! ```sql
//! COPY (SELECT nanoid, game_type, tournament_queen_rule, history, hashes
//!       FROM games
//!       WHERE game_type = 'Base+MLP' AND history <> '' AND array_length(hashes, 1) > 0
//!       ORDER BY created_at) TO STDOUT WITH (FORMAT csv, HEADER);
//!
//! COPY (SELECT nanoid, game_status, conclusion, finished
//!       FROM games
//!       WHERE game_type = 'Base+MLP' AND history <> '' AND array_length(hashes, 1) > 0
//!       ORDER BY created_at) TO STDOUT WITH (FORMAT csv, HEADER);
//! ```

use std::{collections::HashMap, env, fs};

use crate::{
    bug::Bug,
    color::Color,
    game_type::GameType,
    history::History,
    piece::Piece,
    state::State,
};

/// The Postgres export quotes fields that contain commas (the hashes array), so splitting
/// lines on ',' would shred rows - this handles CSV quoting properly.
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

/// Mirrors `State::play_and_print`: the tournament flag comes from the opening moves, not from the
/// stored column, so this replays exactly the way the server does.
fn tournament_from(history: &History) -> bool {
    for index in 0..2 {
        if let Some((piece, _)) = history.moves.get(index) {
            if piece.parse::<Piece>().expect("piece").bug() == Bug::Queen {
                return false;
            }
        }
    }
    true
}

fn load(var: &str) -> Vec<Vec<String>> {
    let path = env::var(var).unwrap_or_else(|_| panic!("set {var}"));
    read_csv(&fs::read_to_string(&path).expect("readable CSV"))
}

/// Draws the site awarded that were never threefolds: the old hash keyed the stun by piece type
/// with no cell, so positions restricting different same-type pieces collided.
const RECORDED_BUT_NOT_A_REPETITION: [&str; 1] = ["iIPQLORgQUe9"];

/// Real threefolds recorded as something else. Each verified by [`detected_repetitions_are_real`],
/// which checks this list by default.
const REAL_BUT_RECORDED_OTHERWISE: [&str; 10] = [
    "9DvlsAQscx",
    "pEIOqmy3yl",
    "5xgBXFQQOaYv",
    "kO59eNBcVk8a",
    "oBy-nMA9bw21",
    "CW9_kQ29tO1N",
    "Kg2iccu9y8Dp",
    "C_vgtOD3YAZd",
    "ySoi7lraT6Xc",
    "rsIv-Nbc0xL5",
];

/// The recorded `conclusion` is ground truth - comparing code against code cannot say which
/// side is right.
#[test]
#[ignore = "needs MLP_GAMES_CSV and MLP_RESULTS_CSV"]
fn threefold_detection_matches_recorded_conclusions() {
    let results = load("MLP_RESULTS_CSV");
    let recorded: HashMap<&str, (&str, &str)> = results[1..]
        .iter()
        .filter(|row| row.len() >= 3)
        .map(|row| (row[0].as_str(), (row[1].as_str(), row[2].as_str())))
        .collect();
    let games = load("MLP_GAMES_CSV");

    let (mut joined, mut hit, mut missed, mut extra) = (0, 0, 0, 0);
    let mut missed_examples: Vec<String> = Vec::new();
    let mut extra_examples: Vec<String> = Vec::new();
    let (mut unexplained_missed, mut unexplained_extra): (Vec<String>, Vec<String>) =
        (Vec::new(), Vec::new());

    for row in games[1..].iter() {
        let Some((status, conclusion)) = recorded.get(row[0].as_str()) else {
            continue;
        };
        let Ok(history) = History::new_from_str(&row[3]) else {
            continue;
        };
        let mut state = State::new(GameType::MLP, tournament_from(&history));
        for (piece, pos) in history.moves.iter() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
        }
        joined += 1;

        match (
            *conclusion == "Repetition",
            !state.repeating_moves.is_empty(),
        ) {
            (true, true) => hit += 1,
            (true, false) => {
                missed += 1;
                if missed_examples.len() < 10 {
                    missed_examples.push(format!("{} ({status})", row[0]));
                }
                if !RECORDED_BUT_NOT_A_REPETITION.contains(&row[0].as_str()) {
                    unexplained_missed.push(row[0].clone());
                }
            }
            (false, true) => {
                extra += 1;
                if extra_examples.len() < 20 {
                    extra_examples.push(format!("{} db={conclusion}/{status}", row[0]));
                }
                if !REAL_BUT_RECORDED_OTHERWISE.contains(&row[0].as_str()) {
                    unexplained_extra.push(row[0].clone());
                }
            }
            (false, false) => {}
        }
    }

    println!("games joined: {joined}");
    println!("  recorded Repetition AND detected: {hit}");
    println!("  recorded Repetition BUT missed:   {missed}");
    println!("  detected BUT recorded otherwise:  {extra}");
    for example in &missed_examples {
        println!("    missed: {example}");
    }
    for example in &extra_examples {
        println!("    extra: {example}");
    }

    // Both lists are disagreements with production, and both are asserted exactly. Counting only
    // the misses would let a new false draw slip through as one more line of output.
    assert!(
        unexplained_missed.is_empty(),
        "a recorded threefold draw was not detected: {unexplained_missed:?}"
    );
    assert!(
        unexplained_extra.is_empty(),
        "a repetition we now detect that nobody has verified: {unexplained_extra:?}"
    );
    assert_eq!(
        missed,
        RECORDED_BUT_NOT_A_REPETITION.len(),
        "a known-wrong draw stopped reproducing - if it is genuinely fixed, drop it from the list"
    );
    assert_eq!(
        extra,
        REAL_BUT_RECORDED_OTHERWISE.len(),
        "a known missed repetition stopped reproducing - if genuinely fixed, drop it from the list"
    );
}

/// An oracle owing nothing to the hash: identical layout and legal moves. Defaults to
/// [`REAL_BUT_RECORDED_OTHERWISE`], so that allowlist is checked rather than trusted.
#[test]
#[ignore = "needs MLP_GAMES_CSV"]
fn detected_repetitions_are_real() {
    let wanted: Vec<String> = match env::var("GAMES") {
        Ok(list) => list.split(',').map(str::to_string).collect(),
        Err(_) => REAL_BUT_RECORDED_OTHERWISE
            .iter()
            .map(|id| id.to_string())
            .collect(),
    };
    let games = load("MLP_GAMES_CSV");
    let mut bogus = Vec::new();
    let mut seen = 0usize;

    for row in games[1..].iter().filter(|r| wanted.contains(&r[0])) {
        seen += 1;
        let history = History::new_from_str(&row[3]).expect("history");

        // Pass one: what does the engine claim?
        let mut state = State::new(GameType::MLP, tournament_from(&history));
        for (piece, pos) in history.moves.iter() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
        }
        let claimed = state.repeating_moves.clone();
        if claimed.len() < 3 {
            bogus.push(format!("{}: only {} occurrences", row[0], claimed.len()));
            continue;
        }

        // Pass two: capture the position and the legal moves at each claimed ply.
        let mut state = State::new(GameType::MLP, tournament_from(&history));
        let mut snapshots: Vec<(Vec<String>, Vec<String>)> = Vec::new();
        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
            if !claimed.contains(&ply) {
                continue;
            }
            let mut layout: Vec<String> = state
                .board
                .positions
                .iter()
                .enumerate()
                .filter_map(|(offset, maybe)| {
                    let at = (*maybe)?;
                    let piece = state.board.offset_to_piece(offset);
                    Some(format!("{}@{},{}", piece.simple(), at.q, at.r))
                })
                .collect();
            layout.sort();
            let to_move = if ply.is_multiple_of(2) {
                Color::Black
            } else {
                Color::White
            };
            let mut legal: Vec<String> = state
                .board
                .moves(to_move)
                .into_iter()
                .flat_map(|((p, _), targets)| {
                    targets
                        .into_iter()
                        .map(move |t| format!("{}->{},{}", p.simple(), t.q, t.r))
                })
                .collect();
            legal.sort();
            snapshots.push((layout, legal));
        }

        let same_layout = snapshots.windows(2).all(|w| w[0].0 == w[1].0);
        let same_moves = snapshots.windows(2).all(|w| w[0].1 == w[1].1);
        println!(
            "{}: {} occurrences at {:?} - layout {}, legal moves {}",
            row[0],
            snapshots.len(),
            claimed,
            if same_layout { "IDENTICAL" } else { "DIFFER" },
            if same_moves { "IDENTICAL" } else { "DIFFER" }
        );
        if !(same_layout && same_moves) {
            bogus.push(row[0].clone());
        }
    }

    assert!(bogus.is_empty(), "not a real repetition: {bogus:?}");
    // A typo'd nanoid, or an export that no longer carries these games, would otherwise verify
    // nothing and say so in green.
    assert_eq!(
        seen,
        wanted.len(),
        "only {seen} of {} requested games were found in the export",
        wanted.len()
    );
}

/// No two distinct positions in the corpus share a hash - checked against the canonical form,
/// which separates a mixing collision from a geometry disagreement.
#[test]
#[ignore = "needs MLP_GAMES_CSV"]
fn canonical_hash_is_position_identity() {
    use crate::canonical_hash::{canonical_form, canonical_hash};

    /// Everything the hash may depend on; the stun sits inside the cells - a separate
    /// `stunned.simple()` term would blind the oracle to the very bug it exists to catch.
    type Form = (Vec<(i32, i32, u32)>, Color);

    let limit: usize = env::var("GAME_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);
    let games = load("MLP_GAMES_CSV");

    let mut by_form: HashMap<Form, u64> = HashMap::new();
    let mut by_hash: HashMap<u64, Form> = HashMap::new();
    let (mut checked, mut split, mut collided) = (0usize, 0usize, 0usize);

    for row in games[1..].iter().take(limit) {
        let Ok(history) = History::new_from_str(&row[3]) else {
            continue;
        };
        let mut state = State::new(GameType::MLP, tournament_from(&history));

        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
            if state.board.played < 2 {
                continue;
            }
            let to_move = if ply.is_multiple_of(2) {
                Color::Black
            } else {
                Color::White
            };
            let Some(cells) = canonical_form(&state.board, state.board.stunned) else {
                continue;
            };
            let form: Form = (cells, to_move);
            let hash = canonical_hash(&state.board, to_move, state.board.stunned);
            checked += 1;

            match by_form.get(&form) {
                Some(&seen) if seen != hash => split += 1,
                None => {
                    by_form.insert(form.clone(), hash);
                }
                _ => {}
            }
            match by_hash.get(&hash) {
                Some(seen) if *seen != form => collided += 1,
                None => {
                    by_hash.insert(hash, form);
                }
                _ => {}
            }
        }
    }

    println!("positions checked:  {checked}");
    println!("distinct positions: {}", by_form.len());
    println!("distinct hashes:    {}", by_hash.len());
    println!("same position, two hashes (split):    {split}");
    println!("two positions, same hash (collision): {collided}");

    assert_eq!(split, 0, "a position must always hash the same");
    assert_eq!(collided, 0, "distinct positions must not share a hash");
    assert_eq!(
        by_form.len(),
        by_hash.len(),
        "positions and hashes must correspond one to one"
    );
}

/// The axis-gap unwrapping must agree with walking the hive, cell for cell. Injectivity alone would
/// not catch geometry that is wrong but consistently wrong.
#[test]
#[ignore = "needs MLP_GAMES_CSV"]
fn axis_unwrap_agrees_with_walking() {
    let games = load("MLP_GAMES_CSV");
    let (mut compared, mut disagreed) = (0usize, 0usize);

    for row in games[1..].iter().take(2000) {
        let Ok(history) = History::new_from_str(&row[3]) else {
            continue;
        };
        let mut state = State::new(GameType::MLP, tournament_from(&history));
        for (piece, pos) in history.moves.iter() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
            if state.board.played < 2 {
                continue;
            }
            compared += 1;
            if !crate::canonical_hash::forms_agree(&state.board, state.board.stunned) {
                disagreed += 1;
            }
        }
    }

    println!("positions compared: {compared}, disagreements: {disagreed}");
    assert_eq!(
        disagreed, 0,
        "axis unwrapping disagrees with walking the hive"
    );
}

/// A deterministic per-game pseudo-random stream (splitmix64), so a failure reproduces exactly.
fn sampler(nanoid: &str) -> impl FnMut() -> u64 {
    let mut state = nanoid.bytes().fold(0xcbf29ce484222325u64, |acc, b| {
        (acc ^ b as u64).wrapping_mul(0x100000001b3)
    });
    move || {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

/// HOP round trips on real games. Re-serialising byte-identically is the load-bearing part:
/// it catches a different board that happens to hash the same.
#[test]
#[ignore = "needs MLP_GAMES_CSV"]
fn hop_round_trips_across_the_corpus() {
    use crate::hop::{from_position, parse, to_hash};

    let limit: usize = env::var("GAME_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    let games = load("MLP_GAMES_CSV");

    let (mut games_checked, mut checked) = (0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();

    for row in games[1..].iter().take(limit) {
        let nanoid = &row[0];
        let Ok(history) = History::new_from_str(&row[3]) else {
            continue;
        };
        if history.moves.is_empty() {
            continue;
        }

        // Three random plies plus the last position of the game.
        let mut next = sampler(nanoid);
        let mut wanted: Vec<usize> = (0..3)
            .map(|_| next() as usize % history.moves.len())
            .collect();
        wanted.push(history.moves.len() - 1);

        let mut state = State::new(GameType::MLP, tournament_from(&history));
        games_checked += 1;

        for (ply, (piece, pos)) in history.moves.iter().enumerate() {
            if state.play_turn_from_history(piece, pos).is_err() {
                break;
            }
            if !wanted.contains(&ply) || state.board.played == 0 {
                continue;
            }
            let to_move = if ply.is_multiple_of(2) {
                Color::Black
            } else {
                Color::White
            };

            let hop = from_position(&state.board, state.game_type, to_move);
            let restored = match parse(&hop) {
                Ok(restored) => restored,
                Err(e) => {
                    failures.push(format!("{nanoid} ply {ply}: {hop}: does not parse: {e}"));
                    continue;
                }
            };
            checked += 1;

            let expected = state.hashes[ply];
            let actual = to_hash(&hop).expect("already parsed") as u64;
            if actual != expected {
                failures.push(format!(
                    "{nanoid} ply {ply}: {hop}: hashed {actual}, played {expected}"
                ));
            }
            let again = from_position(&restored.board, restored.game_type, restored.to_move);
            if again != hop {
                failures.push(format!(
                    "{nanoid} ply {ply}: {hop} re-serialised as {again}"
                ));
            }
            if restored.to_move != to_move || restored.game_type != state.game_type {
                failures.push(format!(
                    "{nanoid} ply {ply}: {hop}: got {:?}/{:?}",
                    restored.game_type, restored.to_move
                ));
            }
        }
    }

    println!(
        "games: {games_checked}, round trips: {checked}, failures: {}",
        failures.len()
    );
    for failure in failures.iter().take(10) {
        println!("  {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} HOP round trips failed",
        failures.len()
    );
}
