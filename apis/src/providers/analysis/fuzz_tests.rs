//! Random action sequences against the analysis store: after every step the selected path must
//! replay to the live state, and the document must round trip.
//!
//! Deep sweep is opt-in:
//! FUZZ_SEQUENCES=2000 FUZZ_SEED=1 cargo test -p apis --release fuzz_tests:: -- --ignored

use super::{document::LoadedAnalysis, store::AnalysisStore, tree::NodeId};
use crate::providers::game_state::{state_hop, GameStateStore, GameStateStoreFields};
use hive_lib::{hop, Bug, GameStatus, GameType, Piece, Position, State};
use leptos::prelude::*;
use std::str::FromStr;

/// SplitMix64, same as the engine's fuzz harness - no dependency needed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn pick(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

fn random_legal_action(state: &State, rng: &mut Rng) -> Option<(Piece, Position)> {
    if matches!(
        state.game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    ) {
        return None;
    }
    let color = state.turn_color;
    let mut options: Vec<(Piece, Position)> = Vec::new();
    for ((piece, _), targets) in state.board.moves(color) {
        for target in targets {
            options.push((piece, target));
        }
    }
    let spawns: Vec<Position> = state.board.spawnable_positions(color).collect();
    for pieces in state.board.reserve(color, state.game_type).values() {
        if let Some(piece) = pieces.first().and_then(|p| Piece::from_str(p).ok()) {
            if piece.bug() == Bug::Queen && !state.queen_allowed() {
                continue;
            }
            if piece.bug() != Bug::Queen && state.queen_required_now(color) {
                continue;
            }
            for &target in &spawns {
                options.push((piece, target));
            }
        }
    }
    (!options.is_empty()).then(|| options[rng.pick(options.len())])
}

/// Mirrors `move_active`, so the fuzzer exercises the production path.
fn play_into_store(store: &AnalysisStore, game_state: GameStateStore, rng: &mut Rng) {
    let action = game_state
        .state()
        .with_untracked(|state| random_legal_action(state, rng));
    let Some((piece, target)) = action else {
        return;
    };
    let appended = game_state
        .state()
        .try_update(|state| {
            let prev = state.history.moves.len();
            state
                .play_turn_from_position(piece, target)
                .expect("a legal option must play");
            state.history.moves[prev..]
                .iter()
                .cloned()
                .zip(state.hashes[prev..].iter().copied())
                .collect::<Vec<_>>()
        })
        .unwrap();
    store.append_moves(appended, game_state);
}

/// The oracle: rebuilding the selected path from the tree reproduces the live state exactly.
fn check_reconstruction(store: &AnalysisStore, game_state: GameStateStore, seed: u64, step: usize) {
    let replayed = store.0.with_untracked(|analysis| {
        analysis
            .arena
            .replay(
                &analysis.selected_path,
                analysis.game_type,
                &analysis.checkpoints,
            )
            .unwrap_or_else(|| panic!("selected path must replay (seed {seed}, step {step})"))
    });
    let live = game_state.state().get_untracked();
    assert_eq!(
        replayed, live,
        "reconstruction diverged from the live state (seed {seed}, step {step})"
    );
    let hop = game_state.state().with_untracked(|state| state_hop(state));
    hop::parse(&hop)
        .unwrap_or_else(|e| panic!("live position must reload: {e} (seed {seed}, step {step})"));
}

fn round_trip_document(
    store: &mut AnalysisStore,
    game_state: GameStateStore,
    seed: u64,
    step: usize,
) {
    let hashes_of = |store: &AnalysisStore| {
        store.0.with_untracked(|analysis| {
            let mut hashes: Vec<(NodeId, Option<u64>)> = analysis
                .arena
                .nodes
                .iter()
                .map(|(id, node)| (*id, node.hash))
                .collect();
            hashes.sort_unstable();
            hashes
        })
    };
    let before = hashes_of(store);
    let selected = store.selected_node_id_untracked();
    let json = store.to_json().unwrap();
    let loaded = LoadedAnalysis::from_json(&json)
        .unwrap_or_else(|e| panic!("document must reload: {e} (seed {seed}, step {step})"));
    game_state.reset_with_state(loaded.playable);
    *store = AnalysisStore::new(loaded.state);
    assert_eq!(
        before,
        hashes_of(store),
        "reload changed node hashes (seed {seed}, step {step})"
    );
    assert_eq!(
        selected,
        store.selected_node_id_untracked(),
        "reload changed the selection (seed {seed}, step {step})"
    );
}

fn random_root(rng: &mut Rng, game_state: GameStateStore) -> AnalysisStore {
    match rng.pick(4) {
        0 => {
            let mut state = State::new(GameType::MLP, false);
            for _ in 0..rng.pick(10) {
                let Some((piece, target)) = random_legal_action(&state, rng) else {
                    break;
                };
                state.play_turn_from_position(piece, target).unwrap();
            }
            let hop = state_hop(&state);
            let loaded = LoadedAnalysis::from_hop(&hop).expect("a reached position loads");
            game_state.reset_with_state(loaded.playable);
            AnalysisStore::new(loaded.state)
        }
        1 => {
            // A long imported record - crosses the checkpoint stride, and replaying past
            // threefolds seeds grandfathered repetitions.
            let mut record = State::new(GameType::MLP, false);
            record.set_replaying(true);
            for _ in 0..40 + rng.pick(50) {
                let Some((piece, target)) = random_legal_action(&record, rng) else {
                    break;
                };
                record.play_turn_from_position(piece, target).unwrap();
            }
            let moves = record.history.moves.clone();
            let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], moves.len())
                .expect("a generated record loads");
            game_state.reset_with_state(loaded.playable);
            AnalysisStore::new(loaded.state)
        }
        _ => AnalysisStore::new_blank(game_state, GameType::MLP),
    }
}

fn run_sequence(seed: u64) {
    let owner = Owner::new();
    owner.with(|| {
        let mut rng = Rng::new(seed);
        let game_state = GameStateStore::new();
        let mut store = random_root(&mut rng, game_state);
        check_reconstruction(&store, game_state, seed, 0);

        for step in 1..=40 {
            match rng.pick(10) {
                // Weighted towards playing, so trees grow enough to navigate and prune.
                0..=4 => play_into_store(&store, game_state, &mut rng),
                5 => {
                    let nodes = store
                        .0
                        .with_untracked(|a| a.arena.nodes.keys().copied().collect::<Vec<_>>());
                    store.select_node(nodes[rng.pick(nodes.len())], game_state);
                }
                6 => {
                    let ply = rng.pick(100);
                    store.select_main_ply(Some(ply), game_state);
                }
                7 => store.promote_current_variation(rng.pick(2) == 0),
                8 => {
                    let nodes = store.0.with_untracked(|a| {
                        a.arena
                            .nodes
                            .keys()
                            .copied()
                            .filter(|id| *id != a.arena.root)
                            .collect::<Vec<_>>()
                    });
                    if !nodes.is_empty() {
                        store.delete_subtree(nodes[rng.pick(nodes.len())], game_state);
                    }
                }
                _ => round_trip_document(&mut store, game_state, seed, step),
            }
            check_reconstruction(&store, game_state, seed, step);
        }
    });
}

#[test]
fn random_analysis_sessions_hold_the_invariants() {
    for seed in 0..60 {
        run_sequence(0xa11ce + seed);
    }
}

#[test]
#[ignore = "deep sweep; set FUZZ_SEQUENCES / FUZZ_SEED"]
fn random_analysis_sessions_hold_the_invariants_deeply() {
    let sequences: u64 = std::env::var("FUZZ_SEQUENCES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000);
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    for sequence in 0..sequences {
        run_sequence(seed.wrapping_add(sequence));
    }
}

/// Uploaded documents are untrusted: a mutated or corrupt JSON must come back as `Err`,
/// never panic - a panic in wasm takes the whole app down.
#[test]
fn hostile_documents_only_error() {
    const JSON_ALPHABET: &[u8] = br#"{}[]",:0123456789 nodeshptw-"#;
    let owner = Owner::new();
    owner.with(|| {
        let mut rng = Rng::new(0xd0c5);
        for round in 0..80u64 {
            let game_state = GameStateStore::new();
            let mut store = random_root(&mut rng, game_state);
            for _ in 0..5 {
                play_into_store(&store, game_state, &mut rng);
            }
            let json = store.to_json().unwrap();
            for _ in 0..12 {
                let mut bytes = json.as_bytes().to_vec();
                for _ in 0..1 + rng.pick(4) {
                    let letter = JSON_ALPHABET[rng.pick(JSON_ALPHABET.len())];
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
                let hostile = String::from_utf8_lossy(&bytes).into_owned();
                let _ = LoadedAnalysis::from_json(&hostile);
            }
            // Silences the unused-assignment warning on the rebind.
            store = AnalysisStore::new_blank(game_state, GameType::MLP);
            let _ = &store;
            let _ = round;
        }
    });
}

/// ANALYSIS_DOC_JSON=../doc.json cargo test -p apis --release exported_document -- --ignored --nocapture
#[test]
#[ignore = "needs ANALYSIS_DOC_JSON pointing at an exported analysis document"]
fn exported_document_reconstructs_every_node() {
    let path = std::env::var("ANALYSIS_DOC_JSON").expect("set ANALYSIS_DOC_JSON");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let analysis = LoadedAnalysis::from_json(&json)
        .expect("document loads")
        .state;
    let no_checkpoints = std::collections::HashMap::new();
    let mut ids: Vec<NodeId> = analysis.arena.nodes.keys().copied().collect();
    ids.sort_unstable();
    let (mut small_nodes, mut big_nodes, mut max_extent, mut deepest) = (0usize, 0usize, 0, 0);
    for id in ids {
        let node_path = analysis.arena.path_to(id).expect("nodes hang off the root");
        let state = analysis
            .arena
            .replay(&node_path, analysis.game_type, &analysis.checkpoints)
            .unwrap_or_else(|| panic!("node {id:?} must replay with checkpoints"));
        let scratch = analysis
            .arena
            .replay(&node_path, analysis.game_type, &no_checkpoints)
            .unwrap_or_else(|| panic!("node {id:?} must replay from scratch"));
        assert_eq!(
            state, scratch,
            "checkpointed reconstruction diverged at node {id:?}"
        );
        let (mut q_min, mut q_max, mut r_min, mut r_max) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for p in state.board.positions.iter().flatten() {
            assert!(
                (2..=30).contains(&p.q) && (2..=30).contains(&p.r),
                "hive hugs the seam at node {id:?}"
            );
            q_min = q_min.min(p.q);
            q_max = q_max.max(p.q);
            r_min = r_min.min(p.r);
            r_max = r_max.max(p.r);
        }
        if state.board.positions.iter().flatten().next().is_some() {
            let extent = (q_max - q_min).max(r_max - r_min);
            max_extent = max_extent.max(extent);
            deepest = deepest.max(state.turn);
            let small = state.board.storage_cells() == 256;
            assert_eq!(
                small,
                extent <= 11,
                "storage does not match the hive extent ({extent}) at node {id:?}"
            );
            if small {
                for p in state.board.positions.iter().flatten() {
                    assert!(
                        (10..=21).contains(&p.q) && (10..=21).contains(&p.r),
                        "small storage but hive outside the window at node {id:?}"
                    );
                }
                small_nodes += 1;
            } else {
                big_nodes += 1;
            }
            hop::parse(&state_hop(&state))
                .unwrap_or_else(|e| panic!("position at node {id:?} must reload: {e}"));
        }
    }
    println!(
        "nodes: {} (small {small_nodes}, big {big_nodes}), max extent {max_extent}, deepest ply {deepest}",
        analysis.arena.nodes.len()
    );
    assert!(
        big_nodes > 0,
        "the document was expected to outgrow the small storage"
    );
}
