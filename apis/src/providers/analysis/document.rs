use super::{
    store::AnalysisState,
    tree::{AnalysisArena, AnalysisNode, MoveDelta, NodeId, PositionCheckpoint, CHECKPOINT_STRIDE},
};
use crate::providers::annotations::AnnotationSet;
use hive_lib::{GameError, GameType, History, State};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};
use thiserror::Error;

pub(super) const ANALYSIS_FORMAT: &str = "hive-analysis";
pub(super) const ANALYSIS_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("JSON error: {0}")]
    Json(String),
    #[error("unsupported analysis document: {0}")]
    Unsupported(String),
    #[error("invalid analysis document: {0}")]
    Invalid(String),
    #[error("invalid move: {0}")]
    Move(String),
}

#[derive(Serialize, Deserialize)]
pub(super) struct AnalysisDocument {
    pub(super) format: String,
    pub(super) version: u32,
    pub(super) game_type: GameType,
    pub(super) root_id: NodeId,
    pub(super) selected_node_id: NodeId,
    pub(super) nodes: Vec<WireNode>,
    #[serde(default)]
    pub(super) annotations: HashMap<NodeId, AnnotationSet>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct WireNode {
    pub(super) id: NodeId,
    pub(super) parent: Option<NodeId>,
    pub(super) children: Vec<NodeId>,
    pub(super) move_delta: Option<MoveDelta>,
    pub(super) position_hash: Option<u64>,
}

pub(super) fn wire_nodes(arena: &AnalysisArena) -> Vec<WireNode> {
    let mut ids = arena.nodes.keys().copied().collect::<Vec<_>>();
    ids.sort_unstable();
    ids.into_iter()
        .map(|id| {
            let node = &arena.nodes[&id];
            WireNode {
                id,
                parent: node.parent,
                children: node.children.clone(),
                move_delta: node.value.clone(),
                position_hash: node.hash,
            }
        })
        .collect()
}

#[derive(Deserialize)]
struct LegacyDocument {
    current_node: Option<LegacyNode>,
    tree: LegacyTree,
    #[serde(default)]
    hashes: HashMap<u64, i32>,
    #[serde(default = "default_game_type")]
    game_type: GameType,
    #[serde(default)]
    annotations: HashMap<i32, AnnotationSet>,
}

#[derive(Deserialize)]
struct LegacyTree {
    nodes: Vec<LegacyNode>,
}

#[derive(Clone, Deserialize)]
struct LegacyNode {
    node_id: i32,
    value: Option<MoveDelta>,
    #[serde(default)]
    children: Vec<i32>,
    parent: Option<i32>,
}

pub(super) struct LoadedAnalysis {
    pub(super) state: AnalysisState,
    pub(super) playable: State,
}

impl LoadedAnalysis {
    pub(super) fn from_moves(
        game_type: GameType,
        moves: &[(String, String)],
        hashes: &[u64],
        selected_count: usize,
    ) -> Result<Self, LoadError> {
        Self::from_linear_moves(game_type, moves, hashes, selected_count, false)
    }

    pub(super) fn from_partial_moves(
        game_type: GameType,
        moves: &[(String, String)],
        hashes: &[u64],
        selected_count: usize,
    ) -> Result<Self, LoadError> {
        Self::from_linear_moves(game_type, moves, hashes, selected_count, true)
    }

    fn from_linear_moves(
        game_type: GameType,
        moves: &[(String, String)],
        hashes: &[u64],
        selected_count: usize,
        keep_valid_prefix: bool,
    ) -> Result<Self, LoadError> {
        if !keep_valid_prefix && selected_count > moves.len() {
            return Err(LoadError::Invalid(
                "selected node does not exist".to_string(),
            ));
        }
        let mut arena = AnalysisArena::blank();
        let mut checkpoints = HashMap::new();
        let mut parent = NodeId::ROOT;
        let mut current_state = State::new(game_type, false);
        let mut selected = (selected_count == 0).then_some(NodeId::ROOT);
        let mut playable = (selected_count == 0).then(|| current_state.clone());
        let mut valid_count = 0;
        for (index, (piece, position)) in moves.iter().enumerate() {
            let depth = index
                .checked_add(1)
                .ok_or_else(|| LoadError::Invalid("raw ply overflow".to_string()))?;
            if let Err(error) = current_state.play_turn_from_history(piece, position) {
                if keep_valid_prefix {
                    break;
                }
                return Err(LoadError::Move(format!(
                    "node {depth}, raw ply {depth}: {error}",
                )));
            }
            let actual_hash = current_state.hashes.last().copied().ok_or_else(|| {
                LoadError::Invalid(format!("node {depth} did not produce a position hash",))
            })?;
            if hashes
                .get(index)
                .is_some_and(|expected_hash| *expected_hash != actual_hash)
            {
                return Err(LoadError::Invalid(format!(
                    "node {depth} has an inconsistent position hash",
                )));
            }
            let id = arena
                .append(
                    parent,
                    MoveDelta {
                        turn: depth,
                        piece: piece.clone(),
                        position: position.clone(),
                    },
                    actual_hash,
                )
                .ok_or_else(|| LoadError::Invalid("node ID or depth overflow".to_string()))?;
            if depth.is_multiple_of(CHECKPOINT_STRIDE) {
                checkpoints.insert(id, PositionCheckpoint::capture(&current_state));
            }
            parent = id;
            valid_count = depth;
            if depth == selected_count {
                selected = Some(id);
                playable = Some(current_state.clone());
            }
        }
        let (selected, playable) = if keep_valid_prefix && selected_count > valid_count {
            (parent, current_state)
        } else {
            (
                selected.ok_or_else(|| {
                    LoadError::Invalid("selected node does not exist".to_string())
                })?,
                playable.ok_or_else(|| {
                    LoadError::Invalid("selected position was not reconstructed".to_string())
                })?,
            )
        };
        let selected_path = arena
            .path_to(selected)
            .ok_or_else(|| LoadError::Invalid("selected node is unreachable".to_string()))?;
        let mut state = AnalysisState {
            arena,
            checkpoints,
            selected_path,
            collapsed: HashSet::new(),
            visible_rows: Vec::new(),
            game_type,
            annotations: HashMap::new(),
            document_generation: 0,
        };
        state.rebuild_visible_rows();
        Ok(Self { state, playable })
    }

    pub(super) fn from_json(input: &str) -> Result<Self, LoadError> {
        let value: serde_json::Value =
            serde_json::from_str(input).map_err(|error| LoadError::Json(error.to_string()))?;
        if value.get("version").is_some() {
            let format = value
                .get("format")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<missing>");
            let version = value.get("version").and_then(serde_json::Value::as_u64);
            if format != ANALYSIS_FORMAT || version != Some(ANALYSIS_VERSION.into()) {
                let version = version
                    .map(|version| version.to_string())
                    .unwrap_or_else(|| "<invalid>".to_string());
                return Err(LoadError::Unsupported(format!(
                    "expected {ANALYSIS_FORMAT} version {ANALYSIS_VERSION}, found {format} version {version}",
                )));
            }
            let document: AnalysisDocument = serde_json::from_value(value)
                .map_err(|error| LoadError::Json(error.to_string()))?;
            Self::from_document(document)
        } else {
            let legacy: LegacyDocument = serde_json::from_value(value)
                .map_err(|error| LoadError::Json(error.to_string()))?;
            Self::from_legacy(legacy)
        }
    }

    fn from_document(document: AnalysisDocument) -> Result<Self, LoadError> {
        let arena = arena_from_wire(document.root_id, document.nodes, true)?;
        Self::validate(
            arena,
            document.selected_node_id,
            document.game_type,
            document.annotations,
            true,
        )
    }

    fn from_legacy(legacy: LegacyDocument) -> Result<Self, LoadError> {
        if legacy.tree.nodes.is_empty() {
            return Err(LoadError::Invalid("legacy tree has no nodes".to_string()));
        }
        let explicit_root = legacy
            .tree
            .nodes
            .iter()
            .find(|node| node.node_id == -1 && node.value.is_none())
            .map(|node| node.node_id);
        let mut id_map = HashMap::new();
        if let Some(root) = explicit_root {
            id_map.insert(root, NodeId::ROOT);
        }
        let mut next_id = 1_u64;
        for node in &legacy.tree.nodes {
            if Some(node.node_id) == explicit_root {
                continue;
            }
            if id_map.insert(node.node_id, NodeId(next_id)).is_some() {
                return Err(LoadError::Invalid(format!(
                    "duplicate legacy node ID {}",
                    node.node_id
                )));
            }
            next_id = next_id
                .checked_add(1)
                .ok_or_else(|| LoadError::Invalid("node ID overflow".to_string()))?;
        }
        let mut hashes_by_node = HashMap::new();
        for (hash, legacy_id) in legacy.hashes {
            if let Some(id) = id_map.get(&legacy_id) {
                hashes_by_node.insert(*id, hash);
            }
        }
        let mut wire_nodes = vec![WireNode {
            id: NodeId::ROOT,
            parent: None,
            children: Vec::new(),
            move_delta: None,
            position_hash: None,
        }];
        for legacy_node in &legacy.tree.nodes {
            if Some(legacy_node.node_id) == explicit_root {
                continue;
            }
            let id = id_map[&legacy_node.node_id];
            let parent = match legacy_node.parent {
                Some(parent) => Some(id_map.get(&parent).copied().ok_or_else(|| {
                    LoadError::Invalid(format!(
                        "legacy node {} has missing parent {parent}",
                        legacy_node.node_id
                    ))
                })?),
                None => Some(NodeId::ROOT),
            };
            let children = legacy_node
                .children
                .iter()
                .map(|child| {
                    id_map.get(child).copied().ok_or_else(|| {
                        LoadError::Invalid(format!(
                            "legacy node {} has missing child {child}",
                            legacy_node.node_id
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            wire_nodes.push(WireNode {
                id,
                parent,
                children,
                move_delta: legacy_node.value.clone(),
                position_hash: hashes_by_node.get(&id).copied(),
            });
        }
        if let Some(root) = explicit_root
            .and_then(|root| legacy.tree.nodes.iter().find(|node| node.node_id == root))
        {
            wire_nodes[0].children = root
                .children
                .iter()
                .map(|child| {
                    id_map.get(child).copied().ok_or_else(|| {
                        LoadError::Invalid(format!("legacy root has missing child {child}"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
        }
        let mut annotations = HashMap::new();
        for (legacy_id, annotation) in legacy.annotations {
            if let Some(id) = id_map.get(&legacy_id) {
                annotations.insert(*id, annotation);
            }
        }
        let selected = match legacy.current_node {
            Some(node) => id_map.get(&node.node_id).copied().ok_or_else(|| {
                LoadError::Invalid(format!(
                    "selected legacy node {} does not exist",
                    node.node_id
                ))
            })?,
            None => NodeId::ROOT,
        };
        let arena = arena_from_wire(NodeId::ROOT, wire_nodes, false)?;
        Self::validate(arena, selected, legacy.game_type, annotations, false)
    }

    pub(super) fn validate(
        mut arena: AnalysisArena,
        selected: NodeId,
        game_type: GameType,
        annotations: HashMap<NodeId, AnnotationSet>,
        require_hashes: bool,
    ) -> Result<Self, LoadError> {
        if !arena.nodes.contains_key(&selected) {
            return Err(LoadError::Invalid(format!(
                "selected node {} does not exist",
                selected.get()
            )));
        }
        if let Some(id) = annotations.keys().find(|id| !arena.nodes.contains_key(id)) {
            return Err(LoadError::Invalid(format!(
                "annotation refers to missing node {}",
                id.get()
            )));
        }
        let mut playable = (selected == arena.root).then(|| State::new(game_type, false));
        let mut checkpoints = HashMap::new();
        let mut stack = vec![(arena.root, State::new(game_type, false))];
        while let Some((parent_id, parent_state)) = stack.pop() {
            let children = arena
                .node(parent_id)
                .map(|node| node.children.clone())
                .ok_or_else(|| LoadError::Invalid("missing traversal node".to_string()))?;
            let child_count = children.len();
            let mut parent_state = Some(parent_state);
            for (index, child_id) in children.into_iter().rev().enumerate() {
                let (depth, delta, expected_hash) = arena
                    .node(child_id)
                    .and_then(|node| {
                        node.value
                            .clone()
                            .map(|value| (node.depth, value, node.hash))
                    })
                    .ok_or_else(|| {
                        LoadError::Invalid(format!("node {} has no move", child_id.get()))
                    })?;
                if require_hashes && expected_hash.is_none() {
                    return Err(LoadError::Invalid(format!(
                        "node {} is missing its position hash",
                        child_id.get()
                    )));
                }
                let mut state = if index + 1 == child_count {
                    parent_state.take().expect("parent state is consumed once")
                } else {
                    parent_state
                        .as_ref()
                        .expect("parent state exists while cloning branches")
                        .clone()
                };
                state
                    .play_turn_from_history(&delta.piece, &delta.position)
                    .map_err(|error| {
                        LoadError::Move(format!(
                            "node {}, raw ply {}: {}",
                            child_id.get(),
                            depth,
                            error
                        ))
                    })?;
                let actual_hash = state.hashes.last().copied().ok_or_else(|| {
                    LoadError::Invalid(format!("node {} did not produce a hash", child_id.get()))
                })?;
                if expected_hash.is_some_and(|hash| hash != actual_hash) {
                    return Err(LoadError::Invalid(format!(
                        "node {} has an inconsistent position hash",
                        child_id.get()
                    )));
                }
                if let Some(node) = arena.nodes.get_mut(&child_id) {
                    node.hash = Some(actual_hash);
                }
                if depth.is_multiple_of(CHECKPOINT_STRIDE) {
                    checkpoints.insert(child_id, PositionCheckpoint::capture(&state));
                }
                if child_id == selected {
                    playable = Some(state.clone());
                }
                stack.push((child_id, state));
            }
        }
        let selected_path = arena
            .path_to(selected)
            .ok_or_else(|| LoadError::Invalid("selected node is unreachable".to_string()))?;
        let collapsed = arena
            .nodes
            .iter()
            .filter_map(|(id, node)| (node.children.len() > 1).then_some(*id))
            .collect();
        let mut state = AnalysisState {
            arena,
            checkpoints,
            selected_path,
            collapsed,
            visible_rows: Vec::new(),
            game_type,
            annotations,
            document_generation: 0,
        };
        state.rebuild_visible_rows();
        Ok(Self {
            state,
            playable: playable.ok_or_else(|| {
                LoadError::Invalid("selected node was not reconstructed".to_string())
            })?,
        })
    }
}

pub(super) fn arena_from_wire(
    root_id: NodeId,
    wire_nodes: Vec<WireNode>,
    strict_children: bool,
) -> Result<AnalysisArena, LoadError> {
    let mut nodes = HashMap::new();
    let mut node_order = Vec::new();
    for wire in wire_nodes {
        let id = wire.id;
        node_order.push(id);
        if nodes
            .insert(
                id,
                AnalysisNode {
                    parent: wire.parent,
                    children: wire.children,
                    value: wire.move_delta,
                    hash: wire.position_hash,
                    depth: 0,
                },
            )
            .is_some()
        {
            return Err(LoadError::Invalid(format!(
                "duplicate node ID {}",
                id.get()
            )));
        }
    }
    let root = nodes
        .get(&root_id)
        .ok_or_else(|| LoadError::Invalid("missing root".to_string()))?;
    if root.parent.is_some() || root.value.is_some() {
        return Err(LoadError::Invalid(
            "root must not have a parent or move".to_string(),
        ));
    }
    if nodes.values().filter(|node| node.parent.is_none()).count() != 1 {
        return Err(LoadError::Invalid(
            "document must have exactly one root".to_string(),
        ));
    }
    let mut derived_children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for id in &node_order {
        let node = &nodes[id];
        if *id == root_id {
            continue;
        }
        let parent = node
            .parent
            .ok_or_else(|| LoadError::Invalid(format!("node {} has no parent", id.get())))?;
        if !nodes.contains_key(&parent) {
            return Err(LoadError::Invalid(format!(
                "node {} has missing parent {}",
                id.get(),
                parent.get()
            )));
        }
        if !strict_children {
            derived_children.entry(parent).or_default().push(*id);
        }
    }
    let legacy_children_are_compact =
        !strict_children && nodes.values().all(|node| node.children.is_empty());
    if legacy_children_are_compact {
        for (parent, children) in derived_children {
            if let Some(node) = nodes.get_mut(&parent) {
                node.children = children;
            }
        }
    }
    let mut owned = HashSet::new();
    for (id, node) in &nodes {
        for child in &node.children {
            if !owned.insert(*child) {
                return Err(LoadError::Invalid(format!(
                    "node {} is owned more than once",
                    child.get()
                )));
            }
            let child_node = nodes
                .get(child)
                .ok_or_else(|| LoadError::Invalid(format!("missing child {}", child.get())))?;
            if child_node.parent != Some(*id) {
                return Err(LoadError::Invalid(format!(
                    "child {} disagrees with parent {}",
                    child.get(),
                    id.get()
                )));
            }
        }
    }
    if owned.len() != nodes.len().saturating_sub(1) {
        return Err(LoadError::Invalid(
            "not every non-root node is owned".to_string(),
        ));
    }
    let mut visited = HashSet::new();
    let mut stack = vec![(root_id, 0_usize)];
    while let Some((id, depth)) = stack.pop() {
        if !visited.insert(id) {
            return Err(LoadError::Invalid(format!(
                "cycle or duplicate ownership at node {}",
                id.get()
            )));
        }
        let node = nodes
            .get_mut(&id)
            .ok_or_else(|| LoadError::Invalid(format!("missing traversal node {}", id.get())))?;
        node.depth = depth;
        let child_depth = depth
            .checked_add(1)
            .ok_or_else(|| LoadError::Invalid("depth overflow".to_string()))?;
        for child in node.children.iter().rev() {
            stack.push((*child, child_depth));
        }
    }
    if visited.len() != nodes.len() {
        return Err(LoadError::Invalid(
            "document contains unreachable nodes".to_string(),
        ));
    }
    for (id, node) in &nodes {
        if *id == root_id {
            continue;
        }
        let value = node
            .value
            .as_ref()
            .ok_or_else(|| LoadError::Invalid(format!("node {} has no move", id.get())))?;
        if value.turn != node.depth {
            return Err(LoadError::Invalid(format!(
                "node {} says raw ply {} but has depth {}",
                id.get(),
                value.turn,
                node.depth
            )));
        }
    }
    let next_id = nodes
        .keys()
        .map(|id| id.get())
        .max()
        .and_then(|id| id.checked_add(1))
        .ok_or_else(|| LoadError::Invalid("node ID overflow".to_string()))?;
    Ok(AnalysisArena {
        root: root_id,
        nodes,
        next_id,
    })
}

fn default_game_type() -> GameType {
    GameType::MLP
}

pub(super) fn requested_raw_ply(raw_ply: Option<usize>, move_count: usize) -> usize {
    raw_ply
        .filter(|ply| *ply <= move_count)
        .unwrap_or(move_count)
}

pub(super) fn parse_uhp(uhp_string: &str) -> Result<History, LoadError> {
    let normalized_uhp = normalize_uhp_metadata(uhp_string);
    match History::parse_uhp_str(&normalized_uhp) {
        Ok(history) => Ok(history),
        Err(GameError::PartialHistory { history, .. }) => Ok(history),
        Err(error) => Err(LoadError::Move(error.to_string())),
    }
}

fn normalize_uhp_metadata(input: &str) -> Cow<'_, str> {
    let mut split = input.splitn(2, ';');
    let header = split.next().unwrap_or_default();
    let rest = split.next();
    if header.starts_with("Base") && header.contains(' ') && !header.contains('+') {
        let normalized_header = header.replace(' ', "+");
        Cow::Owned(match rest {
            Some("") => normalized_header,
            Some(rest) => format!("{normalized_header};{rest}"),
            None => normalized_header,
        })
    } else {
        Cow::Borrowed(input)
    }
}
