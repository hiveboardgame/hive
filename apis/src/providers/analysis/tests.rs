use super::{
    context::{AnalysisContext, AnalysisPreviewSnapshot},
    document::{
        arena_from_wire,
        wire_nodes,
        AnalysisDocument,
        LoadError,
        LoadedAnalysis,
        WireNode,
        ANALYSIS_FORMAT,
        ANALYSIS_VERSION,
    },
    store::{selected_node_from_path, AnalysisStateStoreFields, AnalysisStore},
    tree::{AnalysisArena, AnalysisNode, ChildMatch, MoveDelta, NodeId, PositionCheckpoint},
    view::{build_visible_rows, VisibleRow},
};
use crate::providers::{
    annotations::AnnotationSet,
    game_state::{state_hop, GameStateStore, GameStateStoreFields},
};
use hive_lib::{Color, GameType, State};
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};

fn node(turn: usize, piece: &str, parent: NodeId) -> AnalysisNode {
    AnalysisNode {
        parent: Some(parent),
        children: Vec::new(),
        value: Some(MoveDelta {
            turn,
            piece: piece.to_string(),
            position: String::new(),
        }),
        hash: Some(turn as u64),
        depth: turn,
    }
}

#[test]
fn flat_arena_paths_are_iterative() {
    let mut arena = AnalysisArena::blank();
    let mut parent = NodeId::ROOT;
    for turn in 1..=5_000 {
        let id = NodeId(turn as u64);
        arena.nodes.insert(id, node(turn, "pass", parent));
        arena.nodes.get_mut(&parent).unwrap().children.push(id);
        parent = id;
    }
    assert_eq!(arena.path_to(parent).unwrap().len(), 5_001);
}

#[test]
fn hash_equivalent_sibling_reuses_the_existing_node() {
    let mut arena = AnalysisArena::blank();
    let existing = arena
        .append(
            NodeId::ROOT,
            MoveDelta {
                turn: 1,
                piece: "wA1".to_string(),
                position: String::new(),
            },
            42,
        )
        .unwrap();
    let differently_oriented = MoveDelta {
        turn: 1,
        piece: "wA1".to_string(),
        position: "rotated".to_string(),
    };
    assert_eq!(
        arena.matching_child(NodeId::ROOT, &differently_oriented, 42),
        Some(ChildMatch::Canonical(existing)),
    );
}

#[test]
fn exact_child_match_wins_over_an_earlier_hash_match() {
    let mut arena = AnalysisArena::blank();
    let value = MoveDelta {
        turn: 1,
        piece: "wA1".to_string(),
        position: String::new(),
    };
    let canonical = arena
        .append(
            NodeId::ROOT,
            MoveDelta {
                position: "rotated".to_string(),
                ..value.clone()
            },
            42,
        )
        .unwrap();
    let exact = arena.append(NodeId::ROOT, value.clone(), 7).unwrap();

    assert_ne!(canonical, exact);
    assert_eq!(
        arena.matching_child(NodeId::ROOT, &value, 42),
        Some(ChildMatch::Exact(exact)),
    );
}

#[test]
fn equal_hash_under_a_different_parent_remains_a_distinct_node() {
    let mut arena = AnalysisArena::blank();
    let first_parent = arena
        .append(
            NodeId::ROOT,
            MoveDelta {
                turn: 1,
                piece: "wA1".to_string(),
                position: String::new(),
            },
            1,
        )
        .unwrap();
    let first = arena
        .append(
            first_parent,
            MoveDelta {
                turn: 2,
                piece: "bA1".to_string(),
                position: "-wA1".to_string(),
            },
            42,
        )
        .unwrap();
    let second_parent = arena
        .append(
            NodeId::ROOT,
            MoveDelta {
                turn: 1,
                piece: "wB1".to_string(),
                position: String::new(),
            },
            2,
        )
        .unwrap();
    let value = MoveDelta {
        turn: 2,
        piece: "bB1".to_string(),
        position: "-wB1".to_string(),
    };
    assert_eq!(arena.matching_child(second_parent, &value, 42), None);
    let second = arena.append(second_parent, value, 42).unwrap();
    assert_ne!(first, second);
}

#[test]
fn partial_uhp_loads_its_valid_prefix() {
    for (case, uhp) in [
        (
            "invalid replay suffix",
            "Base;InProgress;White[2];wS1;bS1 wS1-;wQ bad_input;bQ -bS1;wQ wS1/",
        ),
        (
            "parser partial history",
            "Base;InProgress;White[2];wS1;bS1 wS1-;wQ",
        ),
    ] {
        let owner = Owner::new();
        owner.with(|| {
            let game_state = GameStateStore::new();
            let store = AnalysisStore::new_blank(game_state, GameType::MLP);
            store
                .load_uhp(game_state, uhp, None)
                .unwrap_or_else(|error| panic!("{case}: {error}"));

            assert_eq!(
                game_state.state().with_untracked(|state| state.turn),
                2,
                "{case}",
            );
            assert_eq!(store.selected_node_id_untracked(), NodeId(2), "{case}");
            assert_eq!(
                store.0.arena().with_untracked(|arena| arena.nodes.len()),
                3,
                "{case}",
            );
        });
    }
}

#[test]
fn uhp_raw_ply_selection_includes_the_synthetic_root() {
    let owner = Owner::new();
    owner.with(|| {
        let game_state = GameStateStore::new();
        let store = AnalysisStore::new_blank(game_state, GameType::MLP);
        let uhp = "Base;InProgress;White[2];wS1;bS1 wS1-";

        store.load_uhp(game_state, uhp, Some(0)).unwrap();
        assert_eq!(store.selected_node_id_untracked(), NodeId::ROOT);

        store.load_uhp(game_state, uhp, Some(1)).unwrap();
        assert_eq!(store.selected_node_id_untracked(), NodeId(1));

        store.load_uhp(game_state, uhp, Some(99)).unwrap();
        assert_eq!(store.selected_node_id_untracked(), NodeId(2));
    });
}

#[test]
fn document_generation_changes_only_when_the_document_is_replaced() {
    let owner = Owner::new();
    owner.with(|| {
        let game_state = GameStateStore::new();
        let store = AnalysisStore::new_blank(game_state, GameType::MLP);

        assert_eq!(store.document_generation(), 0);

        store
            .load_uhp(game_state, "Base;InProgress;White[2];wS1;bS1 wS1-", None)
            .unwrap();
        assert_eq!(store.document_generation(), 1);

        assert!(store.select_node(NodeId(1), game_state));
        assert_eq!(store.document_generation(), 1);

        store.reset_with_game_type(game_state, GameType::MLP);
        assert_eq!(store.document_generation(), 2);
    });
}

#[test]
fn fast_back_is_disabled_at_the_first_move() {
    let owner = Owner::new();
    owner.with(|| {
        let moves = vec![
            ("wG1".to_string(), String::new()),
            ("bP".to_string(), "\\wG1".to_string()),
        ];
        let first = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 1).unwrap();
        let first_store = AnalysisStore::new(first.state);
        assert_eq!(first_store.first_history_target_node_id(), None);

        let second = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 2).unwrap();
        let second_store = AnalysisStore::new(second.state);
        assert_eq!(second_store.first_history_target_node_id(), Some(NodeId(1)),);
    });
}

#[test]
fn replaying_a_known_child_reuses_its_node() {
    let owner = Owner::new();
    owner.with(|| {
        let moves = vec![
            ("wG1".to_string(), String::new()),
            ("bP".to_string(), "\\wG1".to_string()),
        ];
        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 1).unwrap();
        let existing = NodeId(2);
        let existing_hash = loaded
            .state
            .arena
            .node(existing)
            .and_then(|node| node.hash)
            .unwrap();
        let path = loaded.state.arena.path_to(existing).unwrap();
        let replayed_state = loaded
            .state
            .arena
            .replay(&path, GameType::MLP, &HashMap::new())
            .unwrap();
        let next_id = loaded.state.arena.next_id;
        let game_state = GameStateStore::new();
        game_state.reset_with_state(replayed_state);
        let store = AnalysisStore::new(loaded.state);

        store.append_moves(
            vec![((moves[1].0.clone(), moves[1].1.clone()), existing_hash)],
            game_state,
        );

        assert_eq!(store.selected_node_id_untracked(), existing);
        assert_eq!(
            store.0.selected_path().get_untracked(),
            vec![NodeId::ROOT, NodeId(1), existing],
        );
        store.0.arena().with_untracked(|arena| {
            assert_eq!(arena.nodes.len(), 3);
            assert_eq!(arena.next_id, next_id);
            assert_eq!(arena.node(NodeId(1)).unwrap().children, vec![existing]);
        });
    });
}

#[test]
fn hash_equivalent_child_restores_the_existing_orientation() {
    let owner = Owner::new();
    owner.with(|| {
        let mut existing_state = State::new(GameType::Base, false);
        existing_state.play_turn_from_history("wQ", "").unwrap();
        existing_state.play_turn_from_history("bQ", "-wQ").unwrap();
        let child_position = existing_state
            .board
            .spawnable_positions(Color::White)
            .next()
            .unwrap();
        existing_state
            .play_turn_from_position("wA1".parse().unwrap(), child_position)
            .unwrap();
        let existing_moves = existing_state.history.moves.clone();
        let loaded = LoadedAnalysis::from_moves(GameType::Base, &existing_moves, &[], 1).unwrap();
        let existing = NodeId(2);
        let child = NodeId(3);
        let existing_hash = loaded
            .state
            .arena
            .node(existing)
            .and_then(|node| node.hash)
            .unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);

        game_state
            .state()
            .update(|state| state.play_turn_from_history("bQ", "/wQ").unwrap());
        let alternative_hash = game_state
            .state()
            .with_untracked(|state| state.hashes.last().copied())
            .unwrap();
        assert_eq!(alternative_hash, existing_hash);

        store.append_moves(
            vec![(("bQ".to_string(), "/wQ".to_string()), alternative_hash)],
            game_state,
        );

        assert_eq!(store.selected_node_id_untracked(), existing);
        game_state.state().with_untracked(|state| {
            assert_eq!(state.history.moves, existing_moves[..2]);
        });
        assert!(store.select_node(child, game_state));
        game_state.state().with_untracked(|state| {
            assert_eq!(state.history.moves, existing_moves);
        });
    });
}

/// Convergence is tree-wide: equal depth and equal hash, not just a sibling.
#[test]
fn transposed_move_order_converges_on_the_other_branch() {
    let owner = Owner::new();
    owner.with(|| {
        // Two arms off the queen: the east ant and the north-east grasshopper commute, so the
        // two orders share plies 1-2 and 5, and differ at plies 3-4.
        let line_a = [
            ("wQ", ""),
            ("bQ", "-wQ"),
            ("wA1", "wQ-"),
            ("bA1", "-bQ"),
            ("wG1", "wQ/"),
        ];
        let line_b = [
            ("wQ", ""),
            ("bQ", "-wQ"),
            ("wG1", "wQ/"),
            ("bA1", "-bQ"),
            ("wA1", "wQ-"),
        ];
        let moves_a: Vec<(String, String)> = line_a
            .iter()
            .map(|(piece, position)| (piece.to_string(), position.to_string()))
            .collect();
        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves_a, &[], 0).unwrap();
        let convergence_target = NodeId(5);
        let node_count = loaded.state.arena.nodes.len();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);

        for (piece, position) in line_b {
            let hash = game_state
                .state()
                .try_update(|state| {
                    state.play_turn_from_history(piece, position).unwrap();
                    state.hashes.last().copied().unwrap()
                })
                .unwrap();
            store.append_moves(
                vec![((piece.to_string(), position.to_string()), hash)],
                game_state,
            );
        }

        assert_eq!(store.selected_node_id_untracked(), convergence_target);
        assert_eq!(
            store.0.selected_path().get_untracked(),
            (0..=5).map(NodeId).collect::<Vec<_>>(),
            "selection jumped onto the existing line",
        );
        // Plies 3-4 of the divergent order became real nodes; ply 5 did not.
        store.0.arena().with_untracked(|arena| {
            assert_eq!(arena.nodes.len(), node_count + 2);
        });
        // Notation can differ - the replayer picks its own anchors - so compare hashes.
        let line_hashes: Vec<u64> = store.0.arena().with_untracked(|arena| {
            (1..=5)
                .map(|id| arena.node(NodeId(id)).unwrap().hash.unwrap())
                .collect()
        });
        game_state.state().with_untracked(|state| {
            assert_eq!(state.hashes, line_hashes);
        });
    });
}

/// Promotion reorders children, but `?move=N` still resolves through the imported line.
#[test]
fn select_main_ply_resolves_through_the_imported_game_after_promotion() {
    let owner = Owner::new();
    owner.with(|| {
        let moves: Vec<(String, String)> =
            [("wQ", ""), ("bQ", "-wQ"), ("wA1", "wQ-"), ("bA1", "-bQ")]
                .iter()
                .map(|(piece, position)| (piece.to_string(), position.to_string()))
                .collect();
        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 4).unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);

        // Branch a variation at ply 2 and promote it to the main line.
        assert!(store.select_node(NodeId(2), game_state));
        let hash = game_state
            .state()
            .try_update(|state| {
                state.play_turn_from_history("wG1", "wQ/").unwrap();
                state.hashes.last().copied().unwrap()
            })
            .unwrap();
        store.append_moves(
            vec![(("wG1".to_string(), "wQ/".to_string()), hash)],
            game_state,
        );
        let variation = store.selected_node_id_untracked();
        assert_ne!(variation, NodeId(3));
        store.promote_current_variation(true);
        store.0.arena().with_untracked(|arena| {
            assert_eq!(
                arena.node(NodeId(2)).unwrap().children.first().copied(),
                Some(variation),
                "the variation now leads the first-child chain",
            );
        });

        assert!(store.select_main_ply(Some(3), game_state));
        assert_eq!(
            store.selected_node_id_untracked(),
            NodeId(3),
            "?move=3 is the game's third ply, not the promoted variation",
        );
        assert!(store.select_main_ply(None, game_state));
        assert_eq!(store.selected_node_id_untracked(), NodeId(4));
    });
}

/// An orphaned preview must be undone by the next navigation, never trusted as state.
#[test]
fn navigation_undoes_an_orphaned_explorer_preview() {
    let owner = Owner::new();
    owner.with(|| {
        let moves: Vec<(String, String)> = [("wQ", ""), ("bQ", "-wQ")]
            .iter()
            .map(|(piece, position)| (piece.to_string(), position.to_string()))
            .collect();
        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 1).unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);
        let analysis = AnalysisContext::new(
            store,
            Callback::new(|_| {}),
            Callback::new(|_| {}),
            Callback::new(|_| {}),
        );

        let real = game_state.state().get_untracked();
        analysis.preview.set(Some(AnalysisPreviewSnapshot {
            node_id: store.selected_node_id_untracked(),
            state: real,
            generation: store.document_generation_untracked(),
        }));
        game_state
            .state()
            .update(|state| state.play_turn_from_history("bA1", "wQ-").unwrap());

        // The fast path applies the recorded delta, so without the reset it bakes in the preview.
        assert!(analysis.select_node(NodeId(2), game_state));
        game_state.state().with_untracked(|state| {
            assert_eq!(state.history.moves, moves);
        });
    });
}

#[test]
fn a_preview_from_the_previous_document_is_never_restored() {
    let owner = Owner::new();
    owner.with(|| {
        let moves: Vec<(String, String)> = [("wQ", ""), ("bQ", "-wQ")]
            .iter()
            .map(|(piece, position)| (piece.to_string(), position.to_string()))
            .collect();
        let first = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 2).unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(first.playable);
        let store = AnalysisStore::new(first.state);
        let analysis = AnalysisContext::new(
            store,
            Callback::new(|_| {}),
            Callback::new(|_| {}),
            Callback::new(|_| {}),
        );

        // Hovering an explorer row snapshots the real state; the row then unmounts silently.
        analysis.preview.set(Some(AnalysisPreviewSnapshot {
            node_id: store.selected_node_id_untracked(),
            state: game_state.state().get_untracked(),
            generation: store.document_generation_untracked(),
        }));

        let other: Vec<(String, String)> = [("wS1", ""), ("bS1", "-wS1")]
            .iter()
            .map(|(piece, position)| (piece.to_string(), position.to_string()))
            .collect();
        let second = LoadedAnalysis::from_moves(GameType::MLP, &other, &[], 2).unwrap();
        let json = AnalysisStore::new(second.state).to_json().unwrap();
        store.load_json(game_state, &json).unwrap();
        assert_eq!(
            store.selected_node_id_untracked(),
            NodeId(2),
            "the id must collide, or the test proves nothing"
        );

        analysis.reset_preview(game_state);
        game_state.state().with_untracked(|state| {
            assert_eq!(
                state.history.moves, other,
                "restored the old document's state"
            );
        });
    });
}

#[test]
fn failed_append_does_not_consume_an_id() {
    let mut arena = AnalysisArena::blank();
    let next_id = arena.next_id;
    let delta = MoveDelta {
        turn: 1,
        piece: "wA1".to_string(),
        position: String::new(),
    };

    assert_eq!(arena.append(NodeId(99), delta, 1), None);
    assert_eq!(arena.next_id, next_id);
}

#[test]
fn compact_legacy_document_is_converted_without_tree_ds() {
    let input = serde_json::json!({
        "current_node": {
            "node_id": 1,
            "value": { "turn": 2, "piece": "bP", "position": "\\wG1" },
            "parent": 0
        },
        "tree": {
            "nodes": [
                { "node_id": -1, "value": null, "parent": null },
                {
                    "node_id": 0,
                    "value": { "turn": 1, "piece": "wG1", "position": "" },
                    "parent": -1
                },
                {
                    "node_id": 1,
                    "value": { "turn": 2, "piece": "bP", "position": "\\wG1" },
                    "parent": 0
                }
            ]
        },
        "hashes": {},
        "game_type": "MLP",
        "annotations": {}
    });
    let loaded = LoadedAnalysis::from_json(&input.to_string()).unwrap();
    assert_eq!(loaded.state.arena.nodes.len(), 3);
    assert_eq!(loaded.state.selected_path.len(), 3);
    assert_eq!(loaded.playable.turn, 2);
    assert!(loaded
        .state
        .arena
        .nodes
        .iter()
        .all(|(id, node)| *id == NodeId::ROOT || node.hash.is_some()));
}

#[test]
fn legacy_document_before_synthetic_root_gets_a_root_and_default_game_type() {
    let input = serde_json::json!({
        "current_node": {
            "node_id": 1,
            "value": { "turn": 2, "piece": "bP", "position": "\\wG1" },
            "parent": 0
        },
        "tree": {
            "nodes": [
                {
                    "node_id": 0,
                    "value": { "turn": 1, "piece": "wG1", "position": "" },
                    "parent": null
                },
                {
                    "node_id": 1,
                    "value": { "turn": 2, "piece": "bP", "position": "\\wG1" },
                    "parent": 0
                }
            ]
        },
        "hashes": {},
        "annotations": {}
    });

    let loaded = LoadedAnalysis::from_json(&input.to_string()).unwrap();

    assert_eq!(loaded.state.game_type, GameType::MLP);
    assert_eq!(
        selected_node_from_path(&loaded.state.selected_path),
        NodeId(2),
    );
    assert_eq!(
        loaded.state.arena.node(NodeId::ROOT).unwrap().children,
        vec![NodeId(1)],
    );
    assert_eq!(
        loaded.state.arena.node(NodeId(1)).unwrap().parent,
        Some(NodeId::ROOT),
    );
}

#[test]
fn legacy_document_rejects_a_non_null_missing_parent() {
    let input = serde_json::json!({
        "current_node": null,
        "tree": {
            "nodes": [{
                "node_id": 0,
                "value": { "turn": 1, "piece": "wG1", "position": "" },
                "parent": 99
            }]
        },
        "hashes": {},
        "game_type": "MLP",
        "annotations": {}
    });

    assert!(matches!(
        LoadedAnalysis::from_json(&input.to_string()),
        Err(LoadError::Invalid(message)) if message.contains("missing parent 99")
    ));
}

#[test]
fn versioned_document_round_trip_preserves_ids_selection_and_annotations() {
    let moves = vec![
        ("wG1".to_string(), String::new()),
        ("bP".to_string(), "\\wG1".to_string()),
    ];
    let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 2).unwrap();
    let selected = selected_node_from_path(&loaded.state.selected_path);
    let document = AnalysisDocument {
        format: ANALYSIS_FORMAT.to_string(),
        version: ANALYSIS_VERSION,
        game_type: loaded.state.game_type,
        root_id: loaded.state.arena.root,
        selected_node_id: selected,
        nodes: wire_nodes(&loaded.state.arena),
        annotations: HashMap::from([
            (NodeId::ROOT, AnnotationSet::default()),
            (selected, AnnotationSet::default()),
        ]),
        start_hop: None,
    };
    let json = serde_json::to_string(&document).unwrap();
    let round_trip = LoadedAnalysis::from_json(&json).unwrap();
    assert_eq!(
        selected_node_from_path(&round_trip.state.selected_path),
        selected,
    );
    assert_eq!(round_trip.state.arena.nodes.len(), 3);
    assert!(round_trip.state.annotations.contains_key(&NodeId::ROOT));
    assert!(round_trip.state.annotations.contains_key(&selected));
}

#[test]
fn visible_rows_preserve_layout_and_force_selected_variations_open() {
    let mut arena = AnalysisArena::blank();
    let main = arena
        .append(
            NodeId::ROOT,
            MoveDelta {
                turn: 1,
                piece: "wG1".to_string(),
                position: String::new(),
            },
            1,
        )
        .unwrap();
    let alternate = arena
        .append(
            NodeId::ROOT,
            MoveDelta {
                turn: 1,
                piece: "wA1".to_string(),
                position: String::new(),
            },
            2,
        )
        .unwrap();
    let main_child = arena
        .append(
            main,
            MoveDelta {
                turn: 2,
                piece: "bP".to_string(),
                position: "\\wG1".to_string(),
            },
            3,
        )
        .unwrap();
    let rows = build_visible_rows(
        &arena,
        &HashSet::from([NodeId::ROOT]),
        &[NodeId::ROOT, main, main_child],
    );
    assert_eq!(
        rows,
        vec![
            VisibleRow {
                node_id: NodeId::ROOT,
                indent: 0,
                has_variations: true,
            },
            VisibleRow {
                node_id: main,
                indent: 0,
                has_variations: false,
            },
            VisibleRow {
                node_id: main_child,
                indent: 0,
                has_variations: false,
            },
        ],
    );

    assert_eq!(
        build_visible_rows(
            &arena,
            &HashSet::from([NodeId::ROOT]),
            &[NodeId::ROOT, alternate],
        ),
        vec![
            VisibleRow {
                node_id: NodeId::ROOT,
                indent: 0,
                has_variations: true,
            },
            VisibleRow {
                node_id: alternate,
                indent: 1,
                has_variations: false,
            },
            VisibleRow {
                node_id: main,
                indent: 0,
                has_variations: false,
            },
            VisibleRow {
                node_id: main_child,
                indent: 0,
                has_variations: false,
            },
        ],
    );
}

#[test]
fn versioned_documents_require_a_supported_version() {
    let input = serde_json::json!({
        "format": ANALYSIS_FORMAT,
        "version": ANALYSIS_VERSION + 1,
        "future_schema": true
    });
    assert!(matches!(
        LoadedAnalysis::from_json(&input.to_string()),
        Err(LoadError::Unsupported(_))
    ));
}

#[test]
fn versioned_documents_require_every_move_hash() {
    let nodes = vec![
        WireNode {
            id: NodeId::ROOT,
            parent: None,
            children: vec![NodeId(1)],
            move_delta: None,
            position_hash: None,
        },
        WireNode {
            id: NodeId(1),
            parent: Some(NodeId::ROOT),
            children: Vec::new(),
            move_delta: Some(MoveDelta {
                turn: 1,
                piece: "wG1".to_string(),
                position: String::new(),
            }),
            position_hash: None,
        },
    ];
    let arena = arena_from_wire(NodeId::ROOT, nodes, true).unwrap();
    assert!(matches!(
        LoadedAnalysis::validate(
            arena,
            NodeId(1),
            GameType::MLP,
            HashMap::new(),
            true,
            None
        ),
        Err(LoadError::Invalid(message)) if message.contains("missing its position hash")
    ));
}

#[test]
fn duplicate_child_ownership_is_rejected() {
    let nodes = vec![
        WireNode {
            id: NodeId::ROOT,
            parent: None,
            children: vec![NodeId(1), NodeId(1)],
            move_delta: None,
            position_hash: None,
        },
        WireNode {
            id: NodeId(1),
            parent: Some(NodeId::ROOT),
            children: Vec::new(),
            move_delta: Some(MoveDelta {
                turn: 1,
                piece: "wG1".to_string(),
                position: String::new(),
            }),
            position_hash: Some(1),
        },
    ];
    assert!(matches!(
        arena_from_wire(NodeId::ROOT, nodes, true),
        Err(LoadError::Invalid(message)) if message.contains("owned more than once")
    ));
}

#[test]
fn loaded_unrelated_variations_default_to_collapsed() {
    let input = serde_json::json!({
        "current_node": {
            "node_id": 0,
            "value": { "turn": 1, "piece": "wG1", "position": "" },
            "parent": -1
        },
        "tree": {
            "nodes": [
                { "node_id": -1, "value": null, "parent": null },
                {
                    "node_id": 0,
                    "value": { "turn": 1, "piece": "wG1", "position": "" },
                    "parent": -1
                },
                {
                    "node_id": 1,
                    "value": { "turn": 1, "piece": "wA1", "position": "" },
                    "parent": -1
                }
            ]
        },
        "hashes": {},
        "game_type": "MLP",
        "annotations": {}
    });
    let loaded = LoadedAnalysis::from_json(&input.to_string()).unwrap();
    assert!(loaded.state.collapsed.contains(&NodeId::ROOT));
    assert!(!loaded
        .state
        .visible_rows
        .iter()
        .any(|row| row.node_id == NodeId(2)));
}

#[test]
fn deleting_a_subtree_cleans_node_state_without_reusing_ids() {
    let owner = Owner::new();
    owner.with(|| {
        let moves = vec![
            ("wG1".to_string(), String::new()),
            ("bP".to_string(), "\\wG1".to_string()),
            ("wA1".to_string(), "wG1\\".to_string()),
        ];
        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 3).unwrap();
        let selected = selected_node_from_path(&loaded.state.selected_path);
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);
        let mut survivor = None;
        store.0.arena().update(|arena| {
            survivor = arena.append(
                NodeId(1),
                MoveDelta {
                    turn: 2,
                    piece: "bQ".to_string(),
                    position: "\\wG1".to_string(),
                },
                99,
            );
        });
        let survivor = survivor.unwrap();
        let next_id_before_delete = store.0.arena().with_untracked(|arena| arena.next_id);
        store.0.annotations().update(|annotations| {
            annotations.insert(selected, AnnotationSet::default());
            annotations.insert(NodeId(2), AnnotationSet::default());
        });
        store.0.collapsed().update(|collapsed| {
            collapsed.insert(selected);
            collapsed.insert(NodeId(2));
        });
        store.0.checkpoints().update(|checkpoints| {
            checkpoints.insert(
                selected,
                PositionCheckpoint::capture(&game_state.state().get_untracked()),
            );
        });

        assert!(store.select_node(NodeId(2), game_state));
        let summary = store.selected_subtree_summary().unwrap();
        assert_eq!(summary.node_id, NodeId(2));
        assert_eq!(summary.move_delta.turn, 2);
        assert_eq!(summary.move_delta.piece, "bP");
        assert_eq!(summary.node_count, 2);
        assert!(store.delete_subtree(summary.node_id, game_state));

        assert_eq!(store.selected_node_id_untracked(), NodeId(1));
        store.0.arena().with_untracked(|arena| {
            assert!(!arena.nodes.contains_key(&NodeId(2)));
            assert!(!arena.nodes.contains_key(&selected));
            assert!(arena.nodes.contains_key(&survivor));
        });
        assert!(!store.0.annotations().with_untracked(|annotations| {
            annotations.contains_key(&selected) || annotations.contains_key(&NodeId(2))
        }));
        assert!(!store.0.collapsed().with_untracked(|collapsed| {
            collapsed.contains(&selected) || collapsed.contains(&NodeId(2))
        }));
        assert!(!store
            .0
            .checkpoints()
            .with_untracked(|checkpoints| checkpoints.contains_key(&selected)));
        let mut next = None;
        store.0.arena().update(|arena| {
            next = arena.append(
                NodeId(1),
                MoveDelta {
                    turn: 2,
                    piece: "bA1".to_string(),
                    position: "-wG1".to_string(),
                },
                100,
            );
        });
        assert_eq!(next.unwrap().get(), next_id_before_delete);
    });
}

#[test]
fn each_new_branch_point_gets_expandable_presentation() {
    let owner = Owner::new();
    owner.with(|| {
        let moves = vec![
            ("wG1".to_string(), String::new()),
            ("bP".to_string(), "\\wG1".to_string()),
            ("wA1".to_string(), "wG1\\".to_string()),
        ];
        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 1).unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);

        store.append_moves(
            vec![(("bA1".to_string(), "-wG1".to_string()), 10)],
            game_state,
        );
        assert!(store
            .visible_rows_in(0..usize::MAX)
            .iter()
            .any(|row| row.node_id == NodeId(1) && row.has_variations));

        assert!(store.select_node(NodeId(2), game_state));
        store.append_moves(
            vec![(("wA2".to_string(), "wG1/".to_string()), 11)],
            game_state,
        );

        let rows = store.visible_rows_in(0..usize::MAX);
        assert!(rows
            .iter()
            .any(|row| row.node_id == NodeId(1) && row.has_variations));
        assert!(rows
            .iter()
            .any(|row| row.node_id == NodeId(2) && row.has_variations));
    });
}

#[test]
fn navigation_reconstruction_keeps_analysis_queen_rules() {
    let moves = vec![("wG1".to_string(), String::new())];
    let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 1).unwrap();
    let mut initially_loaded = loaded.playable;
    let path = loaded.state.arena.path_to(NodeId(1)).unwrap();
    let mut reconstructed = loaded
        .state
        .arena
        .replay(&path, GameType::MLP, &HashMap::new())
        .unwrap();

    assert!(!initially_loaded.tournament);
    assert!(!reconstructed.tournament);
    assert!(initially_loaded
        .play_turn_from_history("bQ", "\\wG1")
        .is_ok());
    assert!(reconstructed.play_turn_from_history("bQ", "\\wG1").is_ok());
}

#[test]
fn compact_checkpoint_replays_only_the_remaining_suffix() {
    let moves = vec![
        ("wG1".to_string(), String::new()),
        ("bP".to_string(), "\\wG1".to_string()),
        ("wA1".to_string(), "wG1\\".to_string()),
    ];
    let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 3).unwrap();
    let arena = &loaded.state.arena;
    let checkpoint_path = arena.path_to(NodeId(2)).unwrap();
    let checkpoint_state = arena
        .replay(&checkpoint_path, GameType::MLP, &HashMap::new())
        .unwrap();
    let checkpoints = HashMap::from([(NodeId(2), PositionCheckpoint::capture(&checkpoint_state))]);
    let target_path = arena.path_to(NodeId(3)).unwrap();
    let from_checkpoint = arena
        .replay(&target_path, GameType::MLP, &checkpoints)
        .unwrap();
    let from_root = arena
        .replay(&target_path, GameType::MLP, &HashMap::new())
        .unwrap();

    assert_eq!(from_checkpoint, from_root);
}

/// The root carries a checkpoint holding the loaded position - that is what lets
/// `AnalysisArena::replay` pick it up with no special case of its own.
#[test]
fn hop_rooted_analysis_replays_moves_onto_the_loaded_position() {
    let hop = "base,QA-a,w";
    let loaded = LoadedAnalysis::from_hop(hop).expect("valid HOP");

    assert_eq!(loaded.playable.board.played, 3);
    assert_eq!(loaded.playable.turn_color, Color::White);
    assert!(
        loaded.playable.turn.is_multiple_of(2),
        "White to move implies an even turn"
    );
    assert!(loaded.state.start_hop.is_some(), "the root HOP is recorded");
    assert!(
        loaded
            .state
            .checkpoints
            .contains_key(&loaded.state.arena.root),
        "the root carries a checkpoint holding the loaded position"
    );

    // No ordinary root has a hash: this one has no move to take one from, and the explorer needs it.
    let root_hash = loaded
        .state
        .arena
        .node(loaded.state.arena.root)
        .and_then(|node| node.hash);
    assert_eq!(
        root_hash,
        Some(hive_lib::hop::to_hash(hop).expect("hashable") as u64)
    );

    let replayed = loaded
        .state
        .arena
        .replay(
            &[loaded.state.arena.root],
            loaded.state.game_type,
            &loaded.state.checkpoints,
        )
        .expect("root replays");
    assert_eq!(replayed.board.played, 3);

    let blank = super::store::AnalysisState::blank(loaded.state.game_type);
    let bare = blank
        .arena
        .replay(&[blank.arena.root], blank.game_type, &blank.checkpoints)
        .expect("root replays");
    assert_eq!(bare.board.played, 0);
}

/// The HOP root's occurrence must survive `AnalysisArena::replay` - checkpoints carry no
/// counts, so clicking away and back used to drop the threefold.
#[test]
fn replay_keeps_the_hop_root_counted() {
    let hop = "base,QA-a,w";
    let loaded = LoadedAnalysis::from_hop(hop).expect("valid HOP");
    let root_hash = hive_lib::hop::to_hash(hop).expect("hashable") as u64;

    assert_eq!(
        loaded.playable.hashes_count.get(&root_hash),
        Some(&1),
        "the loaded position counts itself"
    );

    let replayed = loaded
        .state
        .arena
        .replay(
            &[loaded.state.arena.root],
            loaded.state.game_type,
            &loaded.state.checkpoints,
        )
        .expect("root replays");

    assert_eq!(
        replayed.hashes_count, loaded.playable.hashes_count,
        "reconstructing the root must not lose the occurrence it started with"
    );
}

/// The counterpart: an ordinary analysis is rooted at the empty board, which is not a position
/// anyone reached, so there is nothing to count and the root carries no hash to count it by.
#[test]
fn replay_counts_nothing_for_an_ordinary_root() {
    let arena = AnalysisArena::blank();
    let replayed = arena
        .replay(&[arena.root], GameType::MLP, &HashMap::new())
        .expect("root replays");
    assert!(
        replayed.hashes_count.is_empty(),
        "an empty board has not occurred: {:?}",
        replayed.hashes_count
    );
}

/// Trusting a legacy document's saved hashes broke every previously exported analysis the
/// moment the hash algorithm changed. The moves are the record; hashes get recomputed.
#[test]
fn legacy_document_with_stale_hashes_loads_and_recomputes_them() {
    let input = serde_json::json!({
        "current_node": null,
        "tree": {
            "nodes": [
                {
                    "node_id": 0,
                    "value": { "turn": 1, "piece": "wG1", "position": "" },
                    "parent": null
                },
                {
                    "node_id": 1,
                    "value": { "turn": 2, "piece": "bP", "position": "\\wG1" },
                    "parent": 0
                }
            ]
        },
        // Stale on purpose: no hash algorithm ever produced these for this line.
        "hashes": { "11111": 0, "22222": 1 },
        "annotations": {}
    });

    let loaded = LoadedAnalysis::from_json(&input.to_string())
        .expect("stale hashes are derived data, not grounds for rejection");

    let mut state = State::new(GameType::MLP, false);
    state.play_turn_from_history("wG1", "").unwrap();
    let first = *state.hashes.last().unwrap();
    state.play_turn_from_history("bP", "\\wG1").unwrap();
    let second = *state.hashes.last().unwrap();
    assert_eq!(
        loaded.state.arena.node(NodeId(1)).unwrap().hash,
        Some(first)
    );
    assert_eq!(
        loaded.state.arena.node(NodeId(2)).unwrap().hash,
        Some(second)
    );
    assert_eq!(loaded.state.arena.node(NodeId::ROOT).unwrap().hash, None);
}

/// The root's hash is derived from the start HOP, never trusted from the wire - stored root
/// hashes go stale exactly like legacy per-node hashes do.
#[test]
fn versioned_root_hash_is_derived_not_trusted() {
    let hop = "base,QA-a,w";
    let loaded = LoadedAnalysis::from_hop(hop).expect("valid HOP");
    let mut nodes = wire_nodes(&loaded.state.arena);
    // Tamper with the stored root hash, as a stale export after a hash change would.
    for node in nodes.iter_mut() {
        if node.id == loaded.state.arena.root {
            node.position_hash = Some(0xDEAD_BEEF);
        }
    }
    let document = AnalysisDocument {
        format: ANALYSIS_FORMAT.to_string(),
        version: ANALYSIS_VERSION,
        game_type: loaded.state.game_type,
        root_id: loaded.state.arena.root,
        selected_node_id: loaded.state.arena.root,
        nodes,
        annotations: HashMap::new(),
        start_hop: loaded.state.start_hop.clone(),
    };
    let round_trip = LoadedAnalysis::from_json(&serde_json::to_string(&document).unwrap()).unwrap();
    assert_eq!(
        round_trip
            .state
            .arena
            .node(round_trip.state.arena.root)
            .unwrap()
            .hash,
        Some(hive_lib::hop::to_hash(hop).unwrap() as u64),
        "the root hash comes from the HOP, not from the wire"
    );
}

/// We never rotate anything, so claiming an orientation for a board the user has since changed
/// would be a lie.
#[test]
fn the_pasted_orientation_comes_back_only_at_the_root() {
    let loaded = LoadedAnalysis::from_hop("QA-a,w3").unwrap();
    assert_eq!(loaded.state.start_hop.as_deref(), Some("A+a1-Q,w3"));
    assert_eq!(state_hop(&loaded.playable), "A+a1-Q,w");
}

/// A versioned document's start HOP has to agree with the document's own game type - we wrote
/// both, so a mismatch means the file is corrupt.
#[test]
fn versioned_start_hop_must_match_the_document_game_type() {
    let loaded = LoadedAnalysis::from_hop("base,QA-a,w").expect("valid HOP");
    let document = AnalysisDocument {
        format: ANALYSIS_FORMAT.to_string(),
        version: ANALYSIS_VERSION,
        // The HOP says base; the document claims MLP.
        game_type: GameType::MLP,
        root_id: loaded.state.arena.root,
        selected_node_id: loaded.state.arena.root,
        nodes: wire_nodes(&loaded.state.arena),
        annotations: HashMap::new(),
        start_hop: loaded.state.start_hop.clone(),
    };
    let result = LoadedAnalysis::from_json(&serde_json::to_string(&document).unwrap());
    assert!(
        result.is_err(),
        "a game-type mismatch inside a versioned document must be rejected"
    );
}

/// A threefold at ply 6, then both sides place an Ant: a record that continued past a repetition.
/// The final ply is not itself a repetition, so nothing here is a draw.
fn continued_after_repetition() -> Vec<(String, String)> {
    [
        ("wQ", ""),
        ("bQ", "wQ/"),
        ("wQ", "bQ\\"),
        ("bQ", "wQ/"),
        ("wQ", "bQ\\"),
        ("bQ", "wQ/"),
        ("wA1", "-wQ"),
        ("bA1", "bQ-"),
    ]
    .into_iter()
    .map(|(piece, position)| (piece.to_string(), position.to_string()))
    .collect()
}

/// The pure shuffle: ply 7 completes a threefold with nothing recorded after it - the game
/// ended there, so wherever reconstruction hands this position out, it must be a draw.
fn drawn_on_the_final_ply() -> Vec<(String, String)> {
    [
        ("wQ", ""),
        ("bQ", "wQ/"),
        ("wQ", "bQ\\"),
        ("bQ", "wQ/"),
        ("wQ", "bQ\\"),
        ("bQ", "wQ/"),
        ("wQ", "bQ\\"),
    ]
    .into_iter()
    .map(|(piece, position)| (piece.to_string(), position.to_string()))
    .collect()
}

fn document_for(loaded: &LoadedAnalysis) -> AnalysisDocument {
    AnalysisDocument {
        format: ANALYSIS_FORMAT.to_string(),
        version: ANALYSIS_VERSION,
        game_type: loaded.state.game_type,
        root_id: loaded.state.arena.root,
        selected_node_id: selected_node_from_path(&loaded.state.selected_path),
        nodes: wire_nodes(&loaded.state.arena),
        annotations: HashMap::new(),
        start_hop: loaded.state.start_hop.clone(),
    }
}

#[test]
fn analysis_reconstructs_a_history_that_continues_past_a_repetition() {
    let moves = continued_after_repetition();
    let loaded = LoadedAnalysis::from_moves(GameType::Base, &moves, &[], 8)
        .expect("a record that played on must still load");
    assert_eq!(loaded.state.arena.nodes.len(), 9);

    // The final ply is no repetition, so the position is open and play is live again.
    let mut playable = loaded.playable;
    assert_eq!(playable.turn, 8);
    assert_eq!(playable.game_status, hive_lib::GameStatus::InProgress);
    playable
        .play_turn_from_history("wA2", "-wA1")
        .expect("play continues from the reconstructed record");
}

/// The same record through the saved-document path: `validate` replays every branch itself.
#[test]
fn saved_document_with_a_continued_branch_loads() {
    let moves = continued_after_repetition();
    let loaded = LoadedAnalysis::from_moves(GameType::Base, &moves, &[], 8).unwrap();
    let round_trip =
        LoadedAnalysis::from_json(&serde_json::to_string(&document_for(&loaded)).unwrap())
            .expect("a saved analysis that played on must still load");
    assert_eq!(round_trip.state.arena.nodes.len(), 9);
    assert_eq!(
        round_trip.playable.game_status,
        hive_lib::GameStatus::InProgress
    );
}

/// The same record through `AnalysisArena::replay`, which every tree navigation uses.
#[test]
fn replay_walks_a_path_that_continues_past_a_repetition() {
    let moves = continued_after_repetition();
    let loaded = LoadedAnalysis::from_moves(GameType::Base, &moves, &[], 8).unwrap();
    let selected = selected_node_from_path(&loaded.state.selected_path);
    let path = loaded.state.arena.path_to(selected).unwrap();
    let state = loaded
        .state
        .arena
        .replay(&path, GameType::Base, &HashMap::new())
        .expect("navigation must reach every recorded ply");
    assert_eq!(state.turn, 8);
    // The markers survive the trip even though the repetition sits mid-record.
    assert_eq!(state.repeating_moves, vec![1, 3, 5]);
}

/// A UHP that continued past a repetition used to silently truncate at the draw.
#[test]
fn uhp_import_keeps_moves_past_a_repetition() {
    let owner = Owner::new();
    owner.with(|| {
        let game_state = GameStateStore::new();
        let store = AnalysisStore::new_blank(game_state, GameType::Base);
        let uhp = r"Base;InProgress;White[5];wQ;bQ wQ/;wQ bQ\;bQ wQ/;wQ bQ\;bQ wQ/;wA1 -wQ;bA1 bQ-";
        store.load_uhp(game_state, uhp, None).expect("valid UHP");
        assert_eq!(
            store.0.arena().with_untracked(|arena| arena.nodes.len()),
            9,
            "all eight plies load; nothing is truncated at the threefold"
        );
    });
}

/// A threefold with nothing recorded after it ended the game, so reloading must not reopen
/// it. All three reconstruction paths agree.
#[test]
fn a_final_ply_threefold_reloads_as_drawn() {
    let drawn = hive_lib::GameStatus::Finished(hive_lib::GameResult::Draw);
    let moves = drawn_on_the_final_ply();

    let loaded = LoadedAnalysis::from_moves(GameType::Base, &moves, &[], 7).unwrap();
    assert_eq!(loaded.playable.game_status, drawn, "linear import");

    let round_trip =
        LoadedAnalysis::from_json(&serde_json::to_string(&document_for(&loaded)).unwrap()).unwrap();
    assert_eq!(round_trip.playable.game_status, drawn, "saved document");

    let selected = selected_node_from_path(&loaded.state.selected_path);
    let path = loaded.state.arena.path_to(selected).unwrap();
    let state = loaded
        .state
        .arena
        .replay(&path, GameType::Base, &HashMap::new())
        .unwrap();
    assert_eq!(state.game_status, drawn, "navigation replay");
}

/// Stepping child-by-child is navigation of the record too: it must cross a grandfathered
/// repetition instead of adjudicating mid-record and refusing the next recorded move.
#[test]
fn stepping_through_a_grandfathered_repetition_does_not_wedge() {
    let owner = Owner::new();
    owner.with(|| {
        let game_state = GameStateStore::new();
        let store = AnalysisStore::new_blank(game_state, GameType::Base);
        let uhp = r"Base;InProgress;White[5];wQ;bQ wQ/;wQ bQ\;bQ wQ/;wQ bQ\;bQ wQ/;wA1 -wQ;bA1 bQ-";
        store.load_uhp(game_state, uhp, Some(0)).expect("valid UHP");
        assert_eq!(store.selected_node_id_untracked(), NodeId::ROOT);

        for step in 1..=8_u64 {
            assert!(
                store.select_node(NodeId(step), game_state),
                "stepping onto recorded node {step} must work"
            );
        }
        assert_eq!(
            game_state
                .state()
                .with_untracked(|state| state.game_status.clone()),
            hive_lib::GameStatus::InProgress,
            "the record continued, so nothing along it is adjudicated"
        );
    });
}

/// Checkpoints carry neither `repeating_moves` nor the plies before them, so a repetition buried
/// in the checkpointed context used to lose its history markers when navigating beyond it.
#[test]
fn markers_survive_a_checkpoint_that_is_not_itself_repeated() {
    let moves = continued_after_repetition();
    let loaded = LoadedAnalysis::from_moves(GameType::Base, &moves, &[], 8).unwrap();

    // A checkpoint at ply 7: after the repetition, on a position that never repeated.
    let at_seven = LoadedAnalysis::from_moves(GameType::Base, &moves, &[], 7).unwrap();
    let checkpoints = HashMap::from([(NodeId(7), PositionCheckpoint::capture(&at_seven.playable))]);

    let path = loaded.state.arena.path_to(NodeId(8)).unwrap();
    let state = loaded
        .state
        .arena
        .replay(&path, GameType::Base, &checkpoints)
        .expect("checkpointed navigation replays");
    assert_eq!(
        state.repeating_moves,
        vec![1, 3, 5],
        "the repetition markers come from the full hash sequence, not just the replayed tail"
    );
}

/// The adjacent-step shortcut is only an optimisation: when it fails (here the state is already
/// Finished) navigation must fall back to a full replay instead of wedging.
#[test]
fn stepping_falls_back_to_replay_when_the_current_state_is_finished() {
    let owner = Owner::new();
    owner.with(|| {
        let game_state = GameStateStore::new();
        let store = AnalysisStore::new_blank(game_state, GameType::Base);
        let uhp = r"Base;InProgress;White[5];wQ;bQ wQ/;wQ bQ\;bQ wQ/;wQ bQ\;bQ wQ/;wA1 -wQ;bA1 bQ-";
        store.load_uhp(game_state, uhp, Some(6)).expect("valid UHP");

        // Live play can finish the state on a node the record continues past.
        game_state.state().update(|state| {
            state.game_status = hive_lib::GameStatus::Finished(hive_lib::GameResult::Draw);
        });

        assert!(
            store.select_node(NodeId(7), game_state),
            "the shortcut cannot play on a finished state; the replay fallback must"
        );
        assert_eq!(
            game_state
                .state()
                .with_untracked(|state| state.game_status.clone()),
            hive_lib::GameStatus::InProgress,
            "the fallback reconstructed the record's own semantics"
        );
    });
}

/// Canonicalization can renumber pieces, so a continuation recorded against the input frame
/// would point at a different piece on reload.
#[test]
fn hop_rooted_documents_survive_canonicalization() {
    let owner = Owner::new();
    owner.with(|| {
        // `AqQA,w` canonicalizes to `A+QqA,w`, which assigns the Ant numbers from the other end.
        let loaded = LoadedAnalysis::from_hop("AqQA,w").unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);
        let appended = game_state
            .state()
            .try_update(|state| {
                let prev = state.history.moves.len();
                state.play_turn_from_history("wG1", "wA2-").unwrap();
                state.history.moves[prev..]
                    .iter()
                    .cloned()
                    .zip(state.hashes[prev..].iter().copied())
                    .collect::<Vec<_>>()
            })
            .unwrap();
        let saved_hash = appended.first().map(|(_, hash)| *hash);
        store.append_moves(appended, game_state);
        let json = store.to_json().unwrap();
        let reloaded = LoadedAnalysis::from_json(&json)
            .unwrap_or_else(|e| panic!("a saved HOP analysis must reload: {e}"));
        assert_eq!(
            reloaded.state.arena.node(NodeId(1)).and_then(|n| n.hash),
            saved_hash,
            "the continuation must reach the same position after reload",
        );
    });
}

/// The rule reads the board, so this pins the restore bringing the pieces back intact.
#[test]
fn checkpoint_restore_keeps_the_queen_deadline() {
    let loaded = LoadedAnalysis::from_hop("A+G+S-a-g-s,w").unwrap();
    let g2: hive_lib::Piece = "wG2".parse().unwrap();
    let mut live = loaded.playable.clone();
    let spawn = live.board.spawnable_positions(Color::White).next().unwrap();
    assert!(live.play_turn_from_position(g2, spawn).is_err());
    let mut restored = loaded
        .state
        .arena
        .replay(
            &[loaded.state.arena.root],
            loaded.state.game_type,
            &loaded.state.checkpoints,
        )
        .unwrap();
    let spawn = restored
        .board
        .spawnable_positions(Color::White)
        .next()
        .unwrap();
    assert!(restored.play_turn_from_position(g2, spawn).is_err());
}

/// With the root counted, the second return home is a threefold; the draw must survive
/// navigating away and back.
#[test]
fn hop_root_threefold_survives_navigation() {
    use hive_lib::{GameResult, GameStatus};
    let owner = Owner::new();
    owner.with(|| {
        // wA1 - wQ - bQ - bA1 in a row; the end ants swing out and home again.
        let loaded = LoadedAnalysis::from_hop("AQqa,w").unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);
        let wa: hive_lib::Piece = "wA1".parse().unwrap();
        let wq: hive_lib::Piece = "wQ".parse().unwrap();
        let ba: hive_lib::Piece = "bA1".parse().unwrap();
        let bq: hive_lib::Piece = "bQ".parse().unwrap();
        let pos = |piece: hive_lib::Piece| {
            game_state
                .state()
                .with_untracked(|s| s.board.position_of_piece(piece).unwrap())
        };
        let play = |piece: hive_lib::Piece, target: hive_lib::Position| {
            game_state.state().with_untracked(|s| {
                let legal = s
                    .board
                    .moves(s.turn_color)
                    .into_iter()
                    .find(|((p, _), _)| *p == piece)
                    .is_some_and(|(_, targets)| targets.contains(&target));
                assert!(legal, "{piece} -> {target} must be legal");
            });
            let appended = game_state
                .state()
                .try_update(|state| {
                    let prev = state.history.moves.len();
                    state.play_turn_from_position(piece, target).unwrap();
                    state.history.moves[prev..]
                        .iter()
                        .cloned()
                        .zip(state.hashes[prev..].iter().copied())
                        .collect::<Vec<_>>()
                })
                .unwrap();
            store.append_moves(appended, game_state);
        };
        let (wa_home, ba_home) = (pos(wa), pos(ba));
        // Opposite sides of the row, so the dances never touch - whatever orientation the
        // canonical HOP frame loads in.
        let side_cell = |own: hive_lib::Position, enemy: hive_lib::Position, north: bool| {
            game_state.state().with_untracked(|s| {
                let cells = own
                    .positions_around()
                    .filter(|c| !s.board.occupied(*c))
                    .filter(|c| enemy.positions_around().all(|e| e != *c));
                if north {
                    cells.min_by_key(|c| (c.r, c.q)).unwrap()
                } else {
                    cells.max_by_key(|c| (c.r, c.q)).unwrap()
                }
            })
        };
        let wa_away = side_cell(pos(wq), pos(bq), true);
        let ba_away = side_cell(pos(bq), pos(wq), false);
        for _ in 0..2 {
            play(wa, wa_away);
            play(ba, ba_away);
            play(wa, wa_home);
            play(ba, ba_home);
        }
        assert_eq!(
            game_state.state().with_untracked(|s| s.game_status.clone()),
            GameStatus::Finished(GameResult::Draw),
            "the second return to the root is a threefold, live",
        );
        let leaf = store.selected_node_id_untracked();
        assert!(store.select_node(NodeId::ROOT, game_state));
        assert!(store.select_node(leaf, game_state));
        assert_eq!(
            game_state.state().with_untracked(|s| s.game_status.clone()),
            GameStatus::Finished(GameResult::Draw),
            "the draw must survive navigating away and back",
        );
    });
}

/// Deleting the continuation turns the grandfathered repetition node into the line's end;
/// the installed state must show the draw at once, not after the next navigation.
#[test]
fn deleting_past_a_threefold_adjudicates_the_new_leaf() {
    use hive_lib::{Direction, GameResult, GameStatus, State};
    let owner = Owner::new();
    owner.with(|| {
        // Record a 13-ply line: 4 placements, two out-and-home cycles returning to the ply-4
        // position (threefold at ply 12, grandfathered), then one move past it.
        let mut rec = State::new(GameType::MLP, false);
        rec.set_replaying(true);
        for (piece, position) in [("wQ", ""), ("bQ", "-wQ"), ("wA1", "wQ-"), ("bA1", "-bQ")] {
            rec.play_turn_from_history(piece, position).unwrap();
        }
        let wa: hive_lib::Piece = "wA1".parse().unwrap();
        let wq: hive_lib::Piece = "wQ".parse().unwrap();
        let ba: hive_lib::Piece = "bA1".parse().unwrap();
        let bq: hive_lib::Piece = "bQ".parse().unwrap();
        let wa_home = rec.board.position_of_piece(wa).unwrap();
        let ba_home = rec.board.position_of_piece(ba).unwrap();
        let wa_away = rec.board.position_of_piece(wq).unwrap().to(Direction::NE);
        let ba_away = rec.board.position_of_piece(bq).unwrap().to(Direction::SW);
        for _ in 0..2 {
            for (piece, target) in [(wa, wa_away), (ba, ba_away), (wa, wa_home), (ba, ba_home)] {
                rec.play_turn_from_position(piece, target).unwrap();
            }
        }
        rec.play_turn_from_position(wa, wa_away).unwrap();
        let moves = rec.history.moves.clone();
        assert_eq!(moves.len(), 13);

        let loaded = LoadedAnalysis::from_moves(GameType::MLP, &moves, &[], 13).unwrap();
        let game_state = GameStateStore::new();
        game_state.reset_with_state(loaded.playable);
        let store = AnalysisStore::new(loaded.state);
        assert!(store.delete_subtree(NodeId(13), game_state));
        assert_eq!(store.selected_node_id_untracked(), NodeId(12));
        assert_eq!(
            game_state.state().with_untracked(|s| s.game_status.clone()),
            GameStatus::Finished(GameResult::Draw),
            "the repetition node became the line's end; the installed state must show the draw",
        );
    });
}
