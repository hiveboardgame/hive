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
            }
            (false, true) => {
                extra += 1;
                if extra_examples.len() < 20 {
                    extra_examples.push(format!("{} db={conclusion}/{status}", row[0]));
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

    assert_eq!(missed, 0, "a recorded threefold draw was not detected");
}

/// Verify claimed repetitions with an oracle owing nothing to the hash: identical layout
/// (same-type pieces interchangeable) and identical legal moves.
#[test]
#[ignore = "needs MLP_GAMES_CSV and GAMES"]
fn detected_repetitions_are_real() {
    let wanted: Vec<String> = env::var("GAMES")
        .expect("set GAMES to a comma separated list of nanoids")
        .split(',')
        .map(str::to_string)
        .collect();
    let games = load("MLP_GAMES_CSV");
    let mut bogus = Vec::new();

    for row in games[1..].iter().filter(|r| wanted.contains(&r[0])) {
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
}
