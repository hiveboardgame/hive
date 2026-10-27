use super::{
    store::AnalysisState,
    tree::{AnalysisArena, AnalysisNode, MoveDelta, NodeId, PositionCheckpoint, CHECKPOINT_STRIDE},
};
use crate::{i18n::*, providers::annotations::AnnotationSet};
use hive_lib::{hop, GameError, GameStatus, GameType, History, State};
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
    Json(#[from] serde_json::Error),
    #[error("unsupported analysis document: {0}")]
    Unsupported(AnalysisDocumentError),
    #[error("invalid analysis document: {0}")]
    Invalid(AnalysisDocumentError),
    #[error("invalid move: {0}")]
    Move(AnalysisDocumentError),
}

impl LoadError {
    pub fn localized(&self, locale: Locale) -> String {
        match self {
            Self::Json(error) => {
                td_string!(locale, analysis.errors.json, error = error.to_string()).to_string()
            }
            Self::Unsupported(detail) => td_string!(
                locale,
                analysis.errors.unsupported,
                error = detail.localized(locale)
            )
            .to_string(),
            Self::Invalid(detail) => td_string!(
                locale,
                analysis.errors.invalid,
                error = detail.localized(locale)
            )
            .to_string(),
            Self::Move(detail) => td_string!(
                locale,
                analysis.errors.invalid_move,
                error = detail.localized(locale)
            )
            .to_string(),
        }
    }
}

/// Keep validation details structured so an existing import error follows locale changes.
#[derive(Debug, Error)]
pub enum AnalysisDocumentError {
    #[error("{0}")]
    Engine(String),
    #[error("unsupported format/version: {format:?}/{version:?}")]
    UnsupportedVersion {
        format: Option<String>,
        version: Option<u64>,
    },
    #[error("selected node does not exist")]
    SelectedNodeMissing,
    #[error("raw ply overflow")]
    RawPlyOverflow,
    #[error("node {node}, raw ply {ply}: {error}")]
    NodeMove {
        node: String,
        ply: String,
        error: String,
    },
    #[error("node {node} did not produce a position hash")]
    NodeWithoutPositionHash { node: String },
    #[error("node {node} has an inconsistent position hash")]
    InconsistentHash { node: String },
    #[error("node ID or depth overflow")]
    NodeDepthOverflow,
    #[error("selected position was not reconstructed")]
    SelectedPositionNotReconstructed,
    #[error("selected node is unreachable")]
    SelectedNodeUnreachable,
    #[error("the forced pass produced no position hash")]
    ForcedPassWithoutHash,
    #[error("legacy tree has no nodes")]
    EmptyLegacyTree,
    #[error("duplicate legacy node ID {node}")]
    DuplicateLegacyNode { node: String },
    #[error("node ID overflow")]
    NodeIdOverflow,
    #[error("legacy node {node} has missing parent {parent}")]
    LegacyMissingParent { node: String, parent: String },
    #[error("legacy node {node} has missing child {child}")]
    LegacyMissingChild { node: String, child: String },
    #[error("legacy root has missing child {child}")]
    LegacyRootMissingChild { child: String },
    #[error("selected legacy node {node} does not exist")]
    SelectedLegacyNodeMissing { node: String },
    #[error("selected node {node} does not exist")]
    SelectedNodeIdMissing { node: String },
    #[error("annotation refers to missing node {node}")]
    AnnotationNodeMissing { node: String },
    #[error("missing traversal node")]
    MissingTraversalNode,
    #[error("node {node} has no move")]
    NodeWithoutMove { node: String },
    #[error("node {node} is missing its position hash")]
    MissingPositionHash { node: String },
    #[error("node {node} did not produce a hash")]
    NodeWithoutHash { node: String },
    #[error("selected node was not reconstructed")]
    SelectedNodeNotReconstructed,
    #[error("duplicate node ID {node}")]
    DuplicateNode { node: String },
    #[error("missing root")]
    MissingRoot,
    #[error("root must not have a parent or move")]
    InvalidRoot,
    #[error("document must have exactly one root")]
    MultipleRoots,
    #[error("node {node} has no parent")]
    NodeWithoutParent { node: String },
    #[error("node {node} has missing parent {parent}")]
    MissingParent { node: String, parent: String },
    #[error("node {node} is owned more than once")]
    MultipleOwners { node: String },
    #[error("missing child {child}")]
    MissingChild { child: String },
    #[error("child {child} disagrees with parent {parent}")]
    ParentMismatch { child: String, parent: String },
    #[error("not every non-root node is owned")]
    UnownedNode,
    #[error("cycle or duplicate ownership at node {node}")]
    Cycle { node: String },
    #[error("missing traversal node {node}")]
    MissingTraversalNodeId { node: String },
    #[error("depth overflow")]
    DepthOverflow,
    #[error("document contains unreachable nodes")]
    UnreachableNodes,
    #[error("node {node} says raw ply {ply} but has depth {depth}")]
    DepthMismatch {
        node: String,
        ply: String,
        depth: String,
    },
    #[error("start position does not parse: {error}")]
    StartPositionParse { error: String },
    #[error("start position is {actual} but the document says {expected}")]
    StartPositionType { actual: String, expected: String },
    #[error("start position is unreachable: {error}")]
    StartPositionUnreachable { error: String },
}

impl AnalysisDocumentError {
    fn localized(&self, locale: Locale) -> String {
        match self {
            Self::Engine(error) => error.clone(),
            Self::UnsupportedVersion { format, version } => td_string!(
                locale,
                analysis.errors.unsupported_version,
                expected_format = ANALYSIS_FORMAT,
                expected_version = ANALYSIS_VERSION,
                format =
                    format
                        .clone()
                        .unwrap_or_else(|| td_string!(locale, analysis.errors.missing).to_string()),
                version = version
                    .map(|version| version.to_string())
                    .unwrap_or_else(
                        || td_string!(locale, analysis.errors.invalid_value).to_string()
                    )
            )
            .to_string(),
            Self::SelectedNodeMissing => {
                td_string!(locale, analysis.errors.selected_node_missing).to_string()
            }
            Self::RawPlyOverflow => {
                td_string!(locale, analysis.errors.raw_ply_overflow).to_string()
            }
            Self::NodeMove { node, ply, error } => td_string!(
                locale,
                analysis.errors.node_move,
                node = node.clone(),
                ply = ply.clone(),
                error = error.clone()
            )
            .to_string(),
            Self::NodeWithoutPositionHash { node } => td_string!(
                locale,
                analysis.errors.node_without_position_hash,
                node = node.clone()
            )
            .to_string(),
            Self::InconsistentHash { node } => td_string!(
                locale,
                analysis.errors.inconsistent_hash,
                node = node.clone()
            )
            .to_string(),
            Self::NodeDepthOverflow => {
                td_string!(locale, analysis.errors.node_depth_overflow).to_string()
            }
            Self::SelectedPositionNotReconstructed => {
                td_string!(locale, analysis.errors.selected_position_not_reconstructed).to_string()
            }
            Self::SelectedNodeUnreachable => {
                td_string!(locale, analysis.errors.selected_node_unreachable).to_string()
            }
            Self::ForcedPassWithoutHash => {
                td_string!(locale, analysis.errors.forced_pass_without_hash).to_string()
            }
            Self::EmptyLegacyTree => {
                td_string!(locale, analysis.errors.empty_legacy_tree).to_string()
            }
            Self::DuplicateLegacyNode { node } => td_string!(
                locale,
                analysis.errors.duplicate_legacy_node,
                node = node.clone()
            )
            .to_string(),
            Self::NodeIdOverflow => {
                td_string!(locale, analysis.errors.node_id_overflow).to_string()
            }
            Self::LegacyMissingParent { node, parent } => td_string!(
                locale,
                analysis.errors.legacy_missing_parent,
                node = node.clone(),
                parent = parent.clone()
            )
            .to_string(),
            Self::LegacyMissingChild { node, child } => td_string!(
                locale,
                analysis.errors.legacy_missing_child,
                node = node.clone(),
                child = child.clone()
            )
            .to_string(),
            Self::LegacyRootMissingChild { child } => td_string!(
                locale,
                analysis.errors.legacy_root_missing_child,
                child = child.clone()
            )
            .to_string(),
            Self::SelectedLegacyNodeMissing { node } => td_string!(
                locale,
                analysis.errors.selected_legacy_node_missing,
                node = node.clone()
            )
            .to_string(),
            Self::SelectedNodeIdMissing { node } => td_string!(
                locale,
                analysis.errors.selected_node_id_missing,
                node = node.clone()
            )
            .to_string(),
            Self::AnnotationNodeMissing { node } => td_string!(
                locale,
                analysis.errors.annotation_node_missing,
                node = node.clone()
            )
            .to_string(),
            Self::MissingTraversalNode => {
                td_string!(locale, analysis.errors.missing_traversal_node).to_string()
            }
            Self::NodeWithoutMove { node } => td_string!(
                locale,
                analysis.errors.node_without_move,
                node = node.clone()
            )
            .to_string(),
            Self::MissingPositionHash { node } => td_string!(
                locale,
                analysis.errors.missing_position_hash,
                node = node.clone()
            )
            .to_string(),
            Self::NodeWithoutHash { node } => td_string!(
                locale,
                analysis.errors.node_without_hash,
                node = node.clone()
            )
            .to_string(),
            Self::SelectedNodeNotReconstructed => {
                td_string!(locale, analysis.errors.selected_node_not_reconstructed).to_string()
            }
            Self::DuplicateNode { node } => {
                td_string!(locale, analysis.errors.duplicate_node, node = node.clone()).to_string()
            }
            Self::MissingRoot => td_string!(locale, analysis.errors.missing_root).to_string(),
            Self::InvalidRoot => td_string!(locale, analysis.errors.invalid_root).to_string(),
            Self::MultipleRoots => td_string!(locale, analysis.errors.multiple_roots).to_string(),
            Self::NodeWithoutParent { node } => td_string!(
                locale,
                analysis.errors.node_without_parent,
                node = node.clone()
            )
            .to_string(),
            Self::MissingParent { node, parent } => td_string!(
                locale,
                analysis.errors.missing_parent,
                node = node.clone(),
                parent = parent.clone()
            )
            .to_string(),
            Self::MultipleOwners { node } => {
                td_string!(locale, analysis.errors.multiple_owners, node = node.clone()).to_string()
            }
            Self::MissingChild { child } => {
                td_string!(locale, analysis.errors.missing_child, child = child.clone()).to_string()
            }
            Self::ParentMismatch { child, parent } => td_string!(
                locale,
                analysis.errors.parent_mismatch,
                child = child.clone(),
                parent = parent.clone()
            )
            .to_string(),
            Self::UnownedNode => td_string!(locale, analysis.errors.unowned_node).to_string(),
            Self::Cycle { node } => {
                td_string!(locale, analysis.errors.cycle, node = node.clone()).to_string()
            }
            Self::MissingTraversalNodeId { node } => td_string!(
                locale,
                analysis.errors.missing_traversal_node_id,
                node = node.clone()
            )
            .to_string(),
            Self::DepthOverflow => td_string!(locale, analysis.errors.depth_overflow).to_string(),
            Self::UnreachableNodes => {
                td_string!(locale, analysis.errors.unreachable_nodes).to_string()
            }
            Self::DepthMismatch { node, ply, depth } => td_string!(
                locale,
                analysis.errors.depth_mismatch,
                node = node.clone(),
                ply = ply.clone(),
                depth = depth.clone()
            )
            .to_string(),
            Self::StartPositionParse { error } => td_string!(
                locale,
                analysis.errors.start_position_parse,
                error = error.clone()
            )
            .to_string(),
            Self::StartPositionType { actual, expected } => td_string!(
                locale,
                analysis.errors.start_position_type,
                actual = actual.clone(),
                expected = expected.clone()
            )
            .to_string(),
            Self::StartPositionUnreachable { error } => td_string!(
                locale,
                analysis.errors.start_position_unreachable,
                error = error.clone()
            )
            .to_string(),
        }
    }
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
    /// The HOP root, when the analysis started from a bare position; `None` is the empty board.
    #[serde(default)]
    pub(super) start_hop: Option<String>,
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
    #[serde(default = "default_game_type")]
    game_type: GameType,
    #[serde(default)]
    annotations: HashMap<i32, AnnotationSet>,
}

#[derive(Deserialize)]
struct LegacyTree {
    nodes: Vec<LegacyNode>,
}

#[derive(Deserialize)]
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
                AnalysisDocumentError::SelectedNodeMissing,
            ));
        }
        let mut arena = AnalysisArena::blank();
        let mut checkpoints = HashMap::new();
        let mut parent = NodeId::ROOT;
        let mut game_line = vec![NodeId::ROOT];
        let mut current_state = State::new(game_type, false);
        // A record that continued past a repetition must still reconstruct (see `State::replaying`).
        current_state.set_replaying(true);
        let mut selected = (selected_count == 0).then_some(NodeId::ROOT);
        let mut playable = None;
        let mut valid_count = 0;
        for (index, (piece, position)) in moves.iter().enumerate() {
            let depth = index
                .checked_add(1)
                .ok_or(LoadError::Invalid(AnalysisDocumentError::RawPlyOverflow))?;
            if let Err(error) = current_state.play_turn_from_history(piece, position) {
                if keep_valid_prefix {
                    break;
                }
                return Err(LoadError::Move(AnalysisDocumentError::NodeMove {
                    node: (depth).to_string(),
                    ply: (depth).to_string(),
                    error: (error).to_string(),
                }));
            }
            let actual_hash = current_state.hashes.last().copied().ok_or_else(|| {
                LoadError::Invalid(AnalysisDocumentError::NodeWithoutPositionHash {
                    node: (depth).to_string(),
                })
            })?;
            if hashes
                .get(index)
                .is_some_and(|expected_hash| *expected_hash != actual_hash)
            {
                return Err(LoadError::Invalid(
                    AnalysisDocumentError::InconsistentHash {
                        node: (depth).to_string(),
                    },
                ));
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
                .ok_or(LoadError::Invalid(AnalysisDocumentError::NodeDepthOverflow))?;
            if depth.is_multiple_of(CHECKPOINT_STRIDE) {
                checkpoints.insert(id, PositionCheckpoint::capture(&current_state));
            }
            parent = id;
            game_line.push(id);
            valid_count = depth;
            if depth == selected_count {
                selected = Some(id);
                if depth < moves.len() {
                    playable = Some(current_state.clone());
                }
            }
        }
        let (selected, mut playable) = if keep_valid_prefix && selected_count > valid_count {
            (parent, current_state)
        } else {
            let selected = selected.ok_or(LoadError::Invalid(
                AnalysisDocumentError::SelectedNodeMissing,
            ))?;
            let playable = if selected_count == valid_count && selected_count > 0 {
                current_state
            } else if selected_count == 0 {
                State::new(game_type, false)
            } else {
                playable.ok_or(LoadError::Invalid(
                    AnalysisDocumentError::SelectedPositionNotReconstructed,
                ))?
            };
            (selected, playable)
        };
        // Play from here is live again, and a threefold with nothing recorded after it ended the
        // game - reloading must not reopen it.
        playable.set_replaying(false);
        let selected_is_leaf = arena
            .node(selected)
            .is_none_or(|node| node.children.is_empty());
        if selected_is_leaf {
            playable.finish_repetition_at_final_ply();
        }
        let selected_path = arena.path_to(selected).ok_or(LoadError::Invalid(
            AnalysisDocumentError::SelectedNodeUnreachable,
        ))?;
        let mut state = AnalysisState {
            arena,
            checkpoints,
            selected_path,
            collapsed: HashSet::new(),
            visible_rows: Vec::new(),
            game_type,
            annotations: HashMap::new(),
            document_generation: 0,
            start_hop: None,
            game_line,
        };
        state.rebuild_visible_rows();
        Ok(Self { state, playable })
    }

    /// The canonical form is stored *and* played from: canonicalization renumbers pieces, and
    /// continuations are recorded against those names, so the input frame would betray them.
    pub(super) fn from_hop(input: &str) -> Result<Self, LoadError> {
        let position = hop::parse(input)
            .map_err(|error| LoadError::Move(AnalysisDocumentError::Engine(error.to_string())))?;
        let canonical = hop::from_position_oriented(
            &position.board,
            position.game_type,
            position.to_move,
            position.orientation,
        );
        let position = hop::parse(&canonical)
            .map_err(|error| LoadError::Move(AnalysisDocumentError::Engine(error.to_string())))?;
        let game_type = position.game_type;
        // An empty board is the ordinary blank analysis, not a start position. A root hash sends
        // the opening explorer to `game_hashes`, which holds no pre-move rows, so the same board
        // would list the openings when blank and nothing when loaded back from its own HOP.
        if position.board.played == 0 {
            let mut state = AnalysisState::blank(game_type);
            state.rebuild_visible_rows();
            return Ok(Self {
                state,
                playable: State::new(game_type, false),
            });
        }
        let mut playable = State::new_from_position(position.board, game_type, position.to_move)
            .map_err(|error| LoadError::Move(AnalysisDocumentError::Engine(error.to_string())))?;

        let mut arena = AnalysisArena::blank();
        // Every other node takes its hash from the move that reached it; the root has no move,
        // and the opening explorer needs one.
        if let Some(node) = arena.nodes.get_mut(&arena.root) {
            node.hash = root_hash(Some(&canonical));
        }
        let mut checkpoints = HashMap::new();
        checkpoints.insert(arena.root, PositionCheckpoint::capture(&playable));

        // Nobody ever submits a pass: `State::play_turn_from_position` auto-passes the *opponent*
        // after a move, and the board only offers piece-and-target actions. So a pasted position
        // whose side to move is shut out - `L!+lQq4=B,b`, ply 9 of `hash/short_pass.pgn` - loads
        // and is then a dead end. The pass goes into the tree, not onto the root, so the root
        // still holds the position that was pasted.
        let mut selected_path = vec![arena.root];
        if !matches!(
            playable.game_status,
            GameStatus::Finished(_) | GameStatus::Adjudicated
        ) && playable.board.is_shutout(playable.turn_color, game_type)
        {
            playable
                .play_turn_from_history("pass", "")
                .map_err(|error| {
                    LoadError::Move(AnalysisDocumentError::Engine(error.to_string()))
                })?;
            let hash = playable.hashes.last().copied().ok_or(LoadError::Invalid(
                AnalysisDocumentError::ForcedPassWithoutHash,
            ))?;
            let root = arena.root;
            let id = arena
                .append(
                    root,
                    MoveDelta {
                        turn: 1,
                        piece: "pass".to_string(),
                        position: String::new(),
                    },
                    hash,
                )
                .ok_or(LoadError::Invalid(AnalysisDocumentError::NodeDepthOverflow))?;
            selected_path.push(id);
        }

        let mut state = AnalysisState {
            selected_path,
            arena,
            checkpoints,
            collapsed: HashSet::new(),
            visible_rows: Vec::new(),
            game_type,
            annotations: HashMap::new(),
            document_generation: 0,
            start_hop: Some(canonical),
            game_line: Vec::new(),
        };
        state.rebuild_visible_rows();
        Ok(Self { state, playable })
    }

    pub(super) fn from_json(input: &str) -> Result<Self, LoadError> {
        let value: serde_json::Value = serde_json::from_str(input)?;
        if value.get("version").is_some() {
            let format = value.get("format").and_then(serde_json::Value::as_str);
            let version = value.get("version").and_then(serde_json::Value::as_u64);
            if format != Some(ANALYSIS_FORMAT) || version != Some(ANALYSIS_VERSION.into()) {
                return Err(LoadError::Unsupported(
                    AnalysisDocumentError::UnsupportedVersion {
                        format: format.map(str::to_owned),
                        version,
                    },
                ));
            }
            let document: AnalysisDocument = serde_json::from_value(value)?;
            Self::from_document(document)
        } else {
            let legacy: LegacyDocument = serde_json::from_value(value)?;
            Self::from_legacy(legacy)
        }
    }

    fn from_document(document: AnalysisDocument) -> Result<Self, LoadError> {
        Self::from_wire(
            document.root_id,
            document.nodes,
            document.selected_node_id,
            document.game_type,
            document.annotations,
            true,
            document.start_hop,
        )
    }

    fn from_legacy(legacy: LegacyDocument) -> Result<Self, LoadError> {
        if legacy.tree.nodes.is_empty() {
            return Err(LoadError::Invalid(AnalysisDocumentError::EmptyLegacyTree));
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
                return Err(LoadError::Invalid(
                    AnalysisDocumentError::DuplicateLegacyNode {
                        node: (node.node_id).to_string(),
                    },
                ));
            }
            next_id = next_id
                .checked_add(1)
                .ok_or(LoadError::Invalid(AnalysisDocumentError::NodeIdOverflow))?;
        }
        // Legacy `hashes` are not even deserialized: pre-algorithm-change values would make
        // `validate` reject every legacy document. The moves are the record.
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
                    LoadError::Invalid(AnalysisDocumentError::LegacyMissingParent {
                        node: (legacy_node.node_id).to_string(),
                        parent: (parent).to_string(),
                    })
                })?),
                None => Some(NodeId::ROOT),
            };
            let children = legacy_node
                .children
                .iter()
                .map(|child| {
                    id_map.get(child).copied().ok_or_else(|| {
                        LoadError::Invalid(AnalysisDocumentError::LegacyMissingChild {
                            node: (legacy_node.node_id).to_string(),
                            child: (child).to_string(),
                        })
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            wire_nodes.push(WireNode {
                id,
                parent,
                children,
                move_delta: legacy_node.value.clone(),
                position_hash: None,
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
                        LoadError::Invalid(AnalysisDocumentError::LegacyRootMissingChild {
                            child: (child).to_string(),
                        })
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
                LoadError::Invalid(AnalysisDocumentError::SelectedLegacyNodeMissing {
                    node: (node.node_id).to_string(),
                })
            })?,
            None => NodeId::ROOT,
        };
        Self::from_wire(
            NodeId::ROOT,
            wire_nodes,
            selected,
            legacy.game_type,
            annotations,
            false,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_wire(
        root_id: NodeId,
        wire_nodes: Vec<WireNode>,
        selected: NodeId,
        game_type: GameType,
        annotations: HashMap<NodeId, AnnotationSet>,
        is_versioned: bool,
        start_hop: Option<String>,
    ) -> Result<Self, LoadError> {
        let arena = arena_from_wire(root_id, wire_nodes, is_versioned)?;
        Self::validate(
            arena,
            selected,
            game_type,
            annotations,
            is_versioned,
            start_hop,
        )
    }

    pub(super) fn validate(
        mut arena: AnalysisArena,
        selected: NodeId,
        game_type: GameType,
        annotations: HashMap<NodeId, AnnotationSet>,
        is_versioned: bool,
        start_hop: Option<String>,
    ) -> Result<Self, LoadError> {
        if !arena.nodes.contains_key(&selected) {
            return Err(LoadError::Invalid(
                AnalysisDocumentError::SelectedNodeIdMissing {
                    node: (selected.get()).to_string(),
                },
            ));
        }
        if let Some(id) = annotations.keys().find(|id| !arena.nodes.contains_key(id)) {
            return Err(LoadError::Invalid(
                AnalysisDocumentError::AnnotationNodeMissing {
                    node: (id.get()).to_string(),
                },
            ));
        }
        // We wrote the versioned root HOP ourselves, so anything `load_root` rejects means the
        // file is corrupt; legacy documents predate HOP roots and carry none.
        let (mut root, derived_root_hash) = load_root(game_type, start_hop.as_deref())?;
        // Derived from the HOP, never trusted from the wire - stored root hashes go stale
        // whenever the hash algorithm changes.
        if let Some(node) = arena.nodes.get_mut(&arena.root) {
            node.hash = derived_root_hash;
        }
        // Parked on the root as a checkpoint: `replay` always starts its checkpoint search at
        // the root, so a HOP-rooted tree needs no special case there.
        let mut checkpoints = HashMap::new();
        if start_hop.is_some() {
            checkpoints.insert(arena.root, PositionCheckpoint::capture(&root));
        }
        // A branch that continued past a repetition must still validate (see `State::replaying`).
        root.set_replaying(true);
        let mut playable = (selected == arena.root).then(|| root.clone());
        let mut stack = vec![(arena.root, root)];
        while let Some((parent_id, parent_state)) = stack.pop() {
            let children = arena
                .node(parent_id)
                .map(|node| node.children.clone())
                .ok_or(LoadError::Invalid(
                    AnalysisDocumentError::MissingTraversalNode,
                ))?;
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
                        LoadError::Invalid(AnalysisDocumentError::NodeWithoutMove {
                            node: (child_id.get()).to_string(),
                        })
                    })?;
                if is_versioned && expected_hash.is_none() {
                    return Err(LoadError::Invalid(
                        AnalysisDocumentError::MissingPositionHash {
                            node: (child_id.get()).to_string(),
                        },
                    ));
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
                        LoadError::Move(AnalysisDocumentError::NodeMove {
                            node: (child_id.get()).to_string(),
                            ply: (depth).to_string(),
                            error: (error).to_string(),
                        })
                    })?;
                let actual_hash = state.hashes.last().copied().ok_or_else(|| {
                    LoadError::Invalid(AnalysisDocumentError::NodeWithoutHash {
                        node: (child_id.get()).to_string(),
                    })
                })?;
                if expected_hash.is_some_and(|hash| hash != actual_hash) {
                    return Err(LoadError::Invalid(
                        AnalysisDocumentError::InconsistentHash {
                            node: (child_id.get()).to_string(),
                        },
                    ));
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
        let selected_path = arena.path_to(selected).ok_or(LoadError::Invalid(
            AnalysisDocumentError::SelectedNodeUnreachable,
        ))?;
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
            start_hop,
            // Documents carry no import provenance, so `?move=N` uses the first-child walk.
            game_line: Vec::new(),
        };
        state.rebuild_visible_rows();
        let mut playable = playable.ok_or(LoadError::Invalid(
            AnalysisDocumentError::SelectedNodeNotReconstructed,
        ))?;
        // Play from here is live again, and a threefold with nothing recorded after it ended the
        // game - reloading must not reopen it.
        playable.set_replaying(false);
        let selected_is_leaf = state
            .arena
            .node(selected)
            .is_none_or(|node| node.children.is_empty());
        if selected_is_leaf {
            playable.finish_repetition_at_final_ply();
        }
        Ok(Self { state, playable })
    }
}

pub(super) fn arena_from_wire(
    root_id: NodeId,
    wire_nodes: Vec<WireNode>,
    is_versioned: bool,
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
            return Err(LoadError::Invalid(AnalysisDocumentError::DuplicateNode {
                node: (id.get()).to_string(),
            }));
        }
    }
    let root = nodes
        .get(&root_id)
        .ok_or(LoadError::Invalid(AnalysisDocumentError::MissingRoot))?;
    if root.parent.is_some() || root.value.is_some() {
        return Err(LoadError::Invalid(AnalysisDocumentError::InvalidRoot));
    }
    if nodes.values().filter(|node| node.parent.is_none()).count() != 1 {
        return Err(LoadError::Invalid(AnalysisDocumentError::MultipleRoots));
    }
    let legacy_children_are_compact =
        !is_versioned && nodes.values().all(|node| node.children.is_empty());
    let mut derived_children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for id in &node_order {
        let node = &nodes[id];
        if *id == root_id {
            continue;
        }
        let parent = node.parent.ok_or_else(|| {
            LoadError::Invalid(AnalysisDocumentError::NodeWithoutParent {
                node: (id.get()).to_string(),
            })
        })?;
        if !nodes.contains_key(&parent) {
            return Err(LoadError::Invalid(AnalysisDocumentError::MissingParent {
                node: (id.get()).to_string(),
                parent: (parent.get()).to_string(),
            }));
        }
        if legacy_children_are_compact {
            derived_children.entry(parent).or_default().push(*id);
        }
    }
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
                return Err(LoadError::Invalid(AnalysisDocumentError::MultipleOwners {
                    node: (child.get()).to_string(),
                }));
            }
            let child_node = nodes.get(child).ok_or_else(|| {
                LoadError::Invalid(AnalysisDocumentError::MissingChild {
                    child: (child.get()).to_string(),
                })
            })?;
            if child_node.parent != Some(*id) {
                return Err(LoadError::Invalid(AnalysisDocumentError::ParentMismatch {
                    child: (child.get()).to_string(),
                    parent: (id.get()).to_string(),
                }));
            }
        }
    }
    if owned.len() != nodes.len().saturating_sub(1) {
        return Err(LoadError::Invalid(AnalysisDocumentError::UnownedNode));
    }
    let mut visited = HashSet::new();
    let mut stack = vec![(root_id, 0_usize)];
    while let Some((id, depth)) = stack.pop() {
        if !visited.insert(id) {
            return Err(LoadError::Invalid(AnalysisDocumentError::Cycle {
                node: (id.get()).to_string(),
            }));
        }
        let node = nodes.get_mut(&id).ok_or_else(|| {
            LoadError::Invalid(AnalysisDocumentError::MissingTraversalNodeId {
                node: (id.get()).to_string(),
            })
        })?;
        node.depth = depth;
        let child_depth = depth
            .checked_add(1)
            .ok_or(LoadError::Invalid(AnalysisDocumentError::DepthOverflow))?;
        for child in node.children.iter().rev() {
            stack.push((*child, child_depth));
        }
    }
    if visited.len() != nodes.len() {
        return Err(LoadError::Invalid(AnalysisDocumentError::UnreachableNodes));
    }
    for (id, node) in &nodes {
        if *id == root_id {
            continue;
        }
        let value = node.value.as_ref().ok_or_else(|| {
            LoadError::Invalid(AnalysisDocumentError::NodeWithoutMove {
                node: (id.get()).to_string(),
            })
        })?;
        if value.turn != node.depth {
            return Err(LoadError::Invalid(AnalysisDocumentError::DepthMismatch {
                node: (id.get()).to_string(),
                ply: (value.turn).to_string(),
                depth: (node.depth).to_string(),
            }));
        }
    }
    let next_id = nodes
        .keys()
        .map(|id| id.get())
        .max()
        .and_then(|id| id.checked_add(1))
        .ok_or(LoadError::Invalid(AnalysisDocumentError::NodeIdOverflow))?;
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
    match History::parse_uhp_str_without_replay(&normalized_uhp) {
        Ok(history) => Ok(history),
        Err(GameError::PartialHistory { history, .. }) => Ok(history),
        Err(error) => Err(LoadError::Move(AnalysisDocumentError::Engine(
            error.to_string(),
        ))),
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

/// Parse, game type, reachability and hash in one operation, so the root state and the root
/// hash always describe the same validated position. Recovering from a bad HOP silently gave
/// the tree an empty board beside the real position's hash.
pub(super) fn load_root(
    game_type: GameType,
    start_hop: Option<&str>,
) -> Result<(State, Option<u64>), LoadError> {
    // Not routed through `,w`: `new_from_position` counts the root occurrence and `new` does
    // not, and every ordinary analysis reads its repetitions off that count.
    let Some(raw) = start_hop else {
        return Ok((State::new(game_type, false), root_hash(None)));
    };
    let position = hop::parse(raw).map_err(|error| {
        LoadError::Invalid(AnalysisDocumentError::StartPositionParse {
            error: (error).to_string(),
        })
    })?;
    if position.game_type != game_type {
        return Err(LoadError::Invalid(
            AnalysisDocumentError::StartPositionType {
                actual: (position.game_type).to_string(),
                expected: (game_type).to_string(),
            },
        ));
    }
    let state = State::new_from_position(position.board, position.game_type, position.to_move)
        .map_err(|error| {
            LoadError::Invalid(AnalysisDocumentError::StartPositionUnreachable {
                error: (error).to_string(),
            })
        })?;
    Ok((state, root_hash(Some(raw))))
}

/// The root has no move to take a hash from, and the opening explorer needs one. The parser
/// rederives any pillbug stun from `!`, so this matches what the engine records.
pub(super) fn root_hash(start_hop: Option<&str>) -> Option<u64> {
    hop::to_hash(start_hop?).ok().map(|hash| hash as u64)
}
