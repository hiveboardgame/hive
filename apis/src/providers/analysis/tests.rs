use super::{
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
    game_state::{GameStateStore, GameStateStoreFields},
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

        store.reset(game_state);
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
            true
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
