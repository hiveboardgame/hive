//! Random legal games, then every serialization and reconstruction boundary must hold.
//! Failures print the seed and the move list, which is all a targeted red test needs.
//!
//! The quick versions run in CI; the deep sweeps are opt-in:
//! FUZZ_GAMES=20000 FUZZ_SEED=1 cargo test --release fuzz_tests:: -- --ignored

use crate::{
    canonical_hash::canonical_hash,
    color::Color,
    game_status::GameStatus,
    game_type::GameType,
    hop,
    piece::Piece,
    position::Position,
    state::State,
};
use std::str::FromStr;

/// SplitMix64: tiny, seedable, good enough for move selection - no dependency needed.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub(crate) fn pick(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

const GAME_TYPES: [GameType; 8] = [
    GameType::Base,
    GameType::M,
    GameType::L,
    GameType::P,
    GameType::ML,
    GameType::LP,
    GameType::MP,
    GameType::MLP,
];

/// One random legal action: a spawn, a move, or the forced pass. False when the game is over
/// or no action exists.
fn play_random_action(state: &mut State, rng: &mut Rng) -> bool {
    if matches!(
        state.game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    ) {
        return false;
    }
    let color = state.turn_color;
    if state.board.is_shutout(color, state.game_type) {
        return state.play_turn_from_history("pass", "").is_ok();
    }
    let mut options: Vec<(Piece, Position)> = Vec::new();
    for ((piece, _), targets) in state.board.moves(color) {
        for target in targets {
            options.push((piece, target));
        }
    }
    let spawns: Vec<Position> = state.board.spawnable_positions(color).collect();
    for pieces in state.board.reserve(color, state.game_type).values() {
        if let Some(piece) = pieces.first().and_then(|p| Piece::from_str(p).ok()) {
            if piece.bug() == crate::bug::Bug::Queen && !state.queen_allowed() {
                continue;
            }
            if piece.bug() != crate::bug::Bug::Queen && state.queen_required_now(color) {
                continue;
            }
            for &target in &spawns {
                options.push((piece, target));
            }
        }
    }
    if options.is_empty() {
        return false;
    }
    let (piece, target) = options[rng.pick(options.len())];
    state
        .play_turn_from_position(piece, target)
        .expect("an option offered by the engine must play");
    true
}

fn random_game(seed: u64) -> State {
    let mut rng = Rng::new(seed);
    let game_type = GAME_TYPES[rng.pick(GAME_TYPES.len())];
    let tournament = rng.pick(2) == 0;
    let mut state = State::new(game_type, tournament);
    // Long enough for the late-game seams: stuns, throws, repetitions.
    let plies = rng.pick(150);
    for _ in 0..plies {
        if !play_random_action(&mut state, &mut rng) {
            break;
        }
    }
    state
}

fn moves_string(state: &State) -> String {
    state
        .history
        .moves
        .iter()
        .map(|(piece, position)| format!("{piece} {position}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn side_to_move(state: &State) -> Color {
    if state.turn.is_multiple_of(2) {
        Color::White
    } else {
        Color::Black
    }
}

fn check_boundaries(state: &State, seed: u64) {
    let context = || format!("seed {seed}, {} [{}]", state.game_type, moves_string(state));

    // Snapshots feed the analysis checkpoints; a lossy one changes behavior after navigation.
    assert_eq!(
        crate::board::Board::from_snapshot(&state.board.snapshot()),
        state.board,
        "snapshot round trip is not the identity ({})",
        context()
    );

    // Copy HOP -> Load HOP: parses, and canonicalization is a fixpoint.
    let hop = hop::from_position(&state.board, state.game_type, side_to_move(state));
    let parsed =
        hop::parse(&hop).unwrap_or_else(|e| panic!("HOP does not reload: {e} ({})", context()));
    let rehop = hop::from_position(&parsed.board, parsed.game_type, parsed.to_move);
    assert_eq!(
        hop,
        rehop,
        "canonical HOP is not a fixpoint ({})",
        context()
    );
    assert_eq!(
        hop::to_hash(&hop).unwrap(),
        hop::to_hash(&rehop).unwrap(),
        "round trip changed the hash ({})",
        context()
    );

    // Undo means "the state minus its last ply" - nothing more.
    if !state.history.moves.is_empty() {
        let mut expected = State::new(state.game_type, state.tournament);
        for (piece, position) in &state.history.moves[..state.history.moves.len() - 1] {
            expected
                .play_turn_from_history(piece, position)
                .unwrap_or_else(|e| panic!("prefix must replay: {e} ({})", context()));
        }
        let mut undone = state.clone();
        undone.undo();
        assert_eq!(
            undone,
            expected,
            "undo is not minus-one-ply ({})",
            context()
        );
    }

    // The recorded history replays under every constructor, to the same position.
    if !state.history.moves.is_empty() {
        let replayed = State::new_from_str(&moves_string(state), &state.game_type.to_string())
            .unwrap_or_else(|e| panic!("history does not replay: {e} ({})", context()));
        assert_eq!(
            canonical_hash(
                &replayed.board,
                side_to_move(&replayed),
                replayed.board.stunned
            ),
            canonical_hash(&state.board, side_to_move(state), state.board.stunned),
            "replay reached a different position ({})",
            context()
        );
    }
}

fn sweep(games: u64, seed: u64) {
    for game in 0..games {
        let state = random_game(seed.wrapping_add(game));
        check_boundaries(&state, seed.wrapping_add(game));
    }
}

#[test]
fn random_games_hold_the_boundaries() {
    sweep(150, 0xd6c0);
}

#[test]
#[ignore = "deep sweep; set FUZZ_GAMES / FUZZ_SEED"]
fn random_games_hold_the_boundaries_deeply() {
    let games: u64 = std::env::var("FUZZ_GAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    sweep(games, seed);
}

/// Mutate a valid input: insert, delete, or replace characters drawn from the format's own
/// alphabet - close-to-valid strings reach far deeper than random bytes.
fn mutate(input: &str, alphabet: &[u8], rng: &mut Rng) -> String {
    let mut bytes = input.as_bytes().to_vec();
    for _ in 0..1 + rng.pick(3) {
        let letter = alphabet[rng.pick(alphabet.len())];
        match rng.pick(3) {
            0 if !bytes.is_empty() => {
                let at = rng.pick(bytes.len());
                bytes[at] = letter;
            }
            1 => {
                let at = rng.pick(bytes.len() + 1);
                bytes.insert(at, letter);
            }
            _ if !bytes.is_empty() => {
                bytes.remove(rng.pick(bytes.len()));
            }
            _ => {}
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Untrusted input - the Load HOP dialog, the `?uhp=` query, uploaded PGN files - must come
/// back as `Err`, never panic: a panic in wasm takes the whole app down.
fn hostile_sweep(rounds: u64, seed: u64) {
    const HOP_ALPHABET: &[u8] = b"AaBbGgSsQqMmLlPpDd0123456789+-!()=,wbmu ltie";
    const MOVE_ALPHABET: &[u8] = b"AaBbGgSsQqMmLlPpwb0123456789-/\\; .pass";
    let mut rng = Rng::new(seed);
    for round in 0..rounds {
        let state = random_game(seed.wrapping_add(round));
        let hop = hop::from_position(&state.board, state.game_type, side_to_move(&state));
        let moves = moves_string(&state);

        let hostile_hop = mutate(&hop, HOP_ALPHABET, &mut rng);
        let _ = hop::parse(&hostile_hop);
        let _ = hop::to_hash(&hostile_hop);

        let hostile_moves = mutate(&moves, MOVE_ALPHABET, &mut rng);
        let uhp = format!("Base+MLP;InProgress;White[2];{hostile_moves}");
        let _ = crate::history::History::from_uhp_str(&uhp);
        let _ = crate::history::History::parse_uhp_str_without_replay(&mutate(
            &uhp,
            MOVE_ALPHABET,
            &mut rng,
        ));
        let pgn = hostile_moves.replace(';', "\n");
        let _ = crate::history::History::from_pgn_str(&pgn);
        let _ = State::new_from_str(&hostile_moves, "Base+MLP");
    }
}

#[test]
fn hostile_inputs_only_error() {
    hostile_sweep(300, 0xbadd);
}

#[test]
#[ignore = "deep sweep; set FUZZ_ROUNDS / FUZZ_SEED"]
fn hostile_inputs_only_error_deeply() {
    let rounds: u64 = std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    hostile_sweep(rounds, seed);
}
