use std::time::Instant;

use hudsoni::{History, State};
use philanthus::{search_outcome, Game, Limits};

const PUZZLES: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/puzzles.csv"));

fn main() {
    let mut args = std::env::args().skip(1);
    let depth: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(4);
    let want_plies: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(5);
    let limit: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(50);

    let mut games = Vec::new();
    for line in PUZZLES.lines() {
        if games.len() >= limit {
            break;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(index) = tokens.iter().position(|token| token.parse::<u32>().is_ok()) else {
            continue;
        };
        if tokens[index].parse::<u32>() != Ok(want_plies) {
            continue;
        }
        let gamestring = tokens[..index].join(" ");
        let Ok(history) = History::from_uhp_str(gamestring) else {
            continue;
        };
        let Ok(state) = State::new_from_history(&history) else {
            continue;
        };
        let game = Game::from_state(&state);
        if game.is_terminal() {
            continue;
        }
        games.push(game);
    }

    let start = Instant::now();
    let mut nodes = 0_u64;
    let mut checksum = 0_i64;
    for game in &games {
        let mut game = game.clone();
        if let Some(outcome) = search_outcome(&mut game, Limits::depth(depth)) {
            nodes += outcome.nodes;
            checksum = checksum.wrapping_add(outcome.score as i64);
        }
    }
    let elapsed = start.elapsed();

    println!(
        "{} positions, depth {depth}: {nodes} nodes in {:.3}s = {:.0} knodes/s ({:.2} ms/pos, checksum {checksum})",
        games.len(),
        elapsed.as_secs_f64(),
        nodes as f64 / elapsed.as_secs_f64() / 1000.0,
        elapsed.as_secs_f64() * 1000.0 / games.len() as f64,
    );
}
