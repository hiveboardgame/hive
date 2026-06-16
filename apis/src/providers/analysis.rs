use crate::{
    providers::{
        annotations::AnnotationSet,
        game_state::{GameState, GameStateSignal},
    },
    responses::GameResponse,
};
use bimap::BiMap;
use hive_lib::{GameError, GameType, History, State};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, vec};
use tree_ds::prelude::{Node, TraversalStrategy, Tree};

use super::game_state;

const START_NODE_ID: i32 = -1;

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}

#[derive(Clone, Copy)]
pub struct AnalysisSignal(pub RwSignal<AnalysisTree>);

/// Annotation key for the root (no current node); real node ids are `>= 0`.
pub const ANALYSIS_ROOT_KEY: i32 = START_NODE_ID;

#[derive(Clone, Serialize, Deserialize)]
pub struct AnalysisTree {
    pub current_node: Option<Node<i32, TreeNode>>,
    pub tree: Tree<i32, TreeNode>,
    pub hashes: BiMap<u64, i32>,
    pub game_type: GameType,
    /// Per-node annotations, keyed by tree node id (or `ANALYSIS_ROOT_KEY`).
    /// Serialized so they survive save/load.
    #[serde(default)]
    pub annotations: HashMap<i32, AnnotationSet>,
}

impl Default for AnalysisTree {
    fn default() -> Self {
        Self::new_with_game_type(GameType::default())
    }
}

impl AnalysisTree {
    fn start_node() -> Node<i32, TreeNode> {
        Node::new(START_NODE_ID, None)
    }

    fn tree_with_start() -> Tree<i32, TreeNode> {
        let mut tree = Tree::new(Some("analysis"));
        let _ = tree.add_node(Self::start_node(), None);
        tree
    }

    fn new_with_game_type(game_type: GameType) -> Self {
        let tree = Self::tree_with_start();
        let current_node = tree.get_node_by_id(&START_NODE_ID);

        Self {
            current_node,
            tree,
            hashes: BiMap::new(),
            game_type,
            annotations: HashMap::new(),
        }
    }

    pub fn is_start_node_id(node_id: i32) -> bool {
        node_id == START_NODE_ID
    }

    pub fn current_node_id(&self) -> Option<i32> {
        self.current_node
            .as_ref()
            .and_then(|node| node.get_node_id().ok())
    }

    pub fn is_at_start(&self) -> bool {
        self.current_node_id().is_some_and(Self::is_start_node_id)
    }

    pub fn first_history_target_node_id(&self) -> Option<i32> {
        self.current_node
            .as_ref()
            .and_then(|node| node.get_node_id().ok())
            .and_then(|node_id| self.first_real_ancestor_id(node_id))
    }

    pub fn next_history_target_node_id(&self) -> Option<i32> {
        self.current_node
            .as_ref()
            .and_then(|node| node.get_children_ids().ok())
            .and_then(|children| children.first().cloned())
    }

    pub fn previous_history_target_node_id(&self) -> Option<i32> {
        self.current_node
            .as_ref()
            .and_then(|node| node.get_parent_id().ok().flatten())
    }

    pub fn has_real_moves(&self) -> bool {
        self.tree
            .get_nodes()
            .iter()
            .any(|node| node.get_value().ok().flatten().is_some())
    }

    pub fn first_real_ancestor_id(&self, node_id: i32) -> Option<i32> {
        self.tree
            .get_ancestor_ids(&node_id)
            .ok()?
            .into_iter()
            .rev()
            .find(|id| {
                self.tree
                    .get_node_by_id(id)
                    .and_then(|node| node.get_value().ok().flatten())
                    .is_some()
            })
    }

    pub fn current_hash(&self) -> u64 {
        self.current_node_id()
            .and_then(|id| self.hashes.get_by_right(&id).copied())
            .unwrap_or(0)
    }

    pub fn ensure_start_node(&mut self) {
        let current_id = self.current_node_id();

        if self
            .tree
            .get_root_node()
            .and_then(|node| node.get_node_id().ok())
            .is_some_and(Self::is_start_node_id)
        {
            if self.current_node.is_none() {
                self.current_node = self.tree.get_node_by_id(&START_NODE_ID);
            }
            return;
        }

        let Some(legacy_root_id) = self
            .tree
            .get_root_node()
            .and_then(|node| node.get_node_id().ok())
        else {
            self.tree = Self::tree_with_start();
            self.current_node = self.tree.get_node_by_id(&START_NODE_ID);
            return;
        };

        let legacy_tree = self.tree.clone();
        let node_ids = legacy_tree
            .traverse(&legacy_root_id, TraversalStrategy::PreOrder)
            .unwrap_or_else(|_| {
                legacy_tree
                    .get_nodes()
                    .iter()
                    .filter_map(|node| node.get_node_id().ok())
                    .collect()
            });

        let mut tree = Self::tree_with_start();
        for node_id in node_ids {
            let Some(node) = legacy_tree.get_node_by_id(&node_id) else {
                continue;
            };
            let value = node.get_value().ok().flatten();
            let parent_id = node.get_parent_id().ok().flatten().unwrap_or(START_NODE_ID);
            let _ = tree.add_node(Node::new(node_id, value), Some(&parent_id));
        }

        self.tree = tree;
        self.current_node = current_id
            .and_then(|id| self.tree.get_node_by_id(&id))
            .or_else(|| self.tree.get_node_by_id(&START_NODE_ID));
    }

    fn next_node_id(&self) -> i32 {
        self.tree
            .get_nodes()
            .iter()
            .filter_map(|node| node.get_node_id().ok())
            .filter(|id| !Self::is_start_node_id(*id))
            .max()
            .map_or(0, |id| id + 1)
    }

    pub fn new_blank_analysis(game_state: GameStateSignal, game_type: GameType) -> Self {
        game_state.signal.update(|gs| {
            *gs = GameState::new_with_game_type(game_type);
            gs.view = game_state::View::Game;
        });

        Self::new_with_game_type(game_type)
    }

    pub fn from_loaded_state(game_state: GameStateSignal, state: &State) -> Self {
        let mut tree = Self::tree_with_start();
        let mut hashes = BiMap::new();
        let mut previous = Some(START_NODE_ID);

        for (i, (piece, position)) in state.history.moves.iter().enumerate() {
            let new_node = Node::new(
                i as i32,
                Some(TreeNode {
                    turn: i + 1,
                    piece: piece.to_string(),
                    position: position.to_string(),
                }),
            );
            if let Ok(new_id) = new_node.get_node_id() {
                let hash = state.history.hashes[i];
                tree.add_node(new_node, previous.as_ref()).ok();
                hashes.insert(hash, new_id);
                previous = Some(new_id);
            }
        }

        let current_node = previous.and_then(|p| tree.get_node_by_id(&p));

        game_state.signal.update(|gs| {
            gs.view = game_state::View::Game;
        });

        Self {
            current_node,
            tree,
            hashes,
            game_type: state.game_type,
            annotations: HashMap::new(),
        }
    }

    pub fn from_game_response(
        game_response: &GameResponse,
        game_state: GameStateSignal,
        move_number: Option<usize>,
    ) -> Option<Self> {
        let state = game_response.create_state();
        let mut tree = Self::tree_with_start();
        let mut hashes = BiMap::new();
        let mut previous = Some(START_NODE_ID);

        for (i, (piece, position)) in state.history.moves.iter().enumerate() {
            let new_node = Node::new(
                i as i32,
                Some(TreeNode {
                    turn: i + 1,
                    piece: piece.to_string(),
                    position: position.to_string(),
                }),
            );
            if let Ok(new_id) = new_node.get_node_id() {
                let hash = state.history.hashes[i];
                tree.add_node(new_node, previous.as_ref()).ok()?;
                hashes.insert(hash, new_id);
                previous = Some(new_id);
            }
        }
        let current_node = previous.and_then(|p| tree.get_node_by_id(&p));
        let mut analysis_tree = Self {
            current_node,
            tree,
            hashes,
            game_type: state.game_type,
            annotations: HashMap::new(),
        };

        let move_count = state.history.moves.len();

        game_state.signal.update(|gs| {
            gs.view = game_state::View::Game;
            gs.game_id = Some(game_response.game_id.clone());
            gs.state = state;
            gs.game_response = Some(game_response.clone());
            gs.black_id = Some(game_response.black_player.uid);
            gs.white_id = Some(game_response.white_player.uid);
        });

        let target_move_id = move_number
            .filter(|move_number| *move_number < move_count)
            .map(|move_number| move_number as i32)
            .unwrap_or_else(|| {
                if move_count == 0 {
                    START_NODE_ID
                } else {
                    move_count.saturating_sub(1) as i32
                }
            });
        if analysis_tree
            .update_node(target_move_id, Some(game_state))
            .is_none()
        {
            analysis_tree.update_node(START_NODE_ID, Some(game_state));
        }
        Some(analysis_tree)
    }

    pub fn from_uhp(
        game_state: GameStateSignal,
        uhp_string: impl Into<String>,
    ) -> Result<Self, GameError> {
        let normalized_uhp = normalize_uhp_metadata(uhp_string);
        let history = match History::from_uhp_str(normalized_uhp) {
            Ok(history) => history,
            Err(GameError::PartialHistory { history, .. }) => history,
            Err(err) => return Err(err),
        };
        let state = State::new_from_history(&history)?;

        game_state.signal.update(|gs| {
            gs.state = state.clone();
            gs.view = game_state::View::Game;
        });

        Ok(Self::from_loaded_state(game_state, &state))
    }

    pub fn update_node(&mut self, node_id: i32, game: Option<GameStateSignal>) -> Option<()> {
        let target_node = self.tree.get_node_by_id(&node_id)?;
        let state = if Self::is_start_node_id(node_id) {
            State::new(self.game_type, false)
        } else {
            let moves = self
                .tree
                .get_ancestor_ids(&node_id)
                .ok()?
                .into_iter()
                .rev()
                .chain(vec![node_id])
                .filter_map(|a| {
                    self.tree
                        .get_node_by_id(&a)
                        .and_then(|node| node.get_value().ok())
                        .flatten()
                        .map(|tree_node| (tree_node.piece, tree_node.position))
                })
                .collect::<Vec<_>>();
            State::new_from_history(&History {
                moves,
                game_type: self.game_type,
                ..History::new()
            })
            .ok()?
        };

        self.current_node = Some(target_node);

        let history_turn = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_value().ok())
            .flatten()
            .map(|v| v.turn);

        if let Some(g) = game {
            g.signal.update(|gs| {
                gs.state = state;
                gs.history_turn = history_turn;
                gs.move_info.reset();
            })
        }
        Some(())
    }

    pub fn add_node(&mut self, last_move: (String, String), hash: u64) {
        self.ensure_start_node();
        let (piece, position) = last_move;
        let turn = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_value().ok())
            .flatten()
            .map_or(1, |v| 1 + v.turn);
        let valid_trasposition = self
            .hashes
            .get_by_left(&hash)
            .and_then(|node_id| self.tree.get_node_by_id(node_id))
            .and_then(
                |node| match (node.get_value().ok().flatten(), node.get_node_id().ok()) {
                    (Some(v), Some(node_id)) if v.turn == turn => self.update_node(node_id, None),
                    _ => None,
                },
            );
        if valid_trasposition.is_some() {
            return;
        }
        let new_id = self.next_node_id();
        let new_node = Node::new(
            new_id,
            Some(TreeNode {
                turn,
                piece,
                position,
            }),
        );
        let parent_id = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_node_id().ok())
            .unwrap_or(START_NODE_ID);

        if self.tree.add_node(new_node, Some(&parent_id)).is_err() {
            return;
        }
        self.hashes.insert(hash, new_id);
        self.current_node = self.tree.get_node_by_id(&new_id);
    }

    /// Annotation key for the current node (or `ANALYSIS_ROOT_KEY` at the root).
    pub fn current_annotation_key(&self) -> i32 {
        self.current_node
            .as_ref()
            .and_then(|node| node.get_node_id().ok())
            .unwrap_or(ANALYSIS_ROOT_KEY)
    }

    pub fn reset(&mut self, game_state: GameStateSignal) {
        self.tree = Self::tree_with_start();
        self.current_node = self.tree.get_node_by_id(&START_NODE_ID);
        self.hashes.clear();
        self.annotations.clear();
        game_state.signal.update(|gs| {
            *gs = GameState::new_with_game_type(self.game_type);
            gs.view = game_state::View::Game;
        });
    }

    // Navigate to the empty board without clearing the analysis tree so the
    // user can go forward again after pressing back past the first move.
    pub fn go_to_start(&mut self, game_state: GameStateSignal) {
        self.ensure_start_node();
        self.current_node = self.tree.get_node_by_id(&START_NODE_ID);
        let empty_state = State::new(self.game_type, false);
        game_state.signal.update(|gs| {
            gs.state = empty_state;
            gs.history_turn = None;
            gs.move_info.reset();
        });
    }
}

fn normalize_uhp_metadata(uhp_string: impl Into<String>) -> String {
    let input = uhp_string.into();
    let mut split = input.splitn(2, ';');
    let header = split.next().unwrap_or_default();
    let rest = split.next();

    // Query parameters decode '+' into ' ', so restore it for the game type metadata token.
    if header.starts_with("Base") && header.contains(' ') && !header.contains('+') {
        let normalized_header = header.replace(' ', "+");
        match rest {
            Some("") => normalized_header,
            Some(rest) => format!("{normalized_header};{rest}"),
            None => normalized_header,
        }
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_node(turn: usize) -> TreeNode {
        TreeNode {
            turn,
            piece: format!("wA{turn}"),
            position: String::new(),
        }
    }

    #[test]
    fn default_analysis_starts_at_empty_board() {
        let analysis = AnalysisTree::default();

        assert_eq!(analysis.current_node_id(), Some(START_NODE_ID));
        assert!(analysis.is_at_start());
        assert!(!analysis.has_real_moves());
        assert_eq!(analysis.current_hash(), 0);
        assert_eq!(
            analysis
                .tree
                .get_root_node()
                .and_then(|node| node.get_node_id().ok()),
            Some(START_NODE_ID)
        );
    }

    #[test]
    fn alternate_first_moves_are_siblings_under_start() {
        let mut analysis = AnalysisTree::default();

        analysis.add_node(("wA1".to_string(), String::new()), 1);
        analysis.current_node = analysis.tree.get_node_by_id(&START_NODE_ID);
        analysis.add_node(("wB1".to_string(), String::new()), 2);

        let start = analysis.tree.get_node_by_id(&START_NODE_ID).unwrap();
        assert_eq!(start.get_children_ids().unwrap(), vec![0, 1]);
        assert_eq!(analysis.hashes.get_by_left(&1), Some(&0));
        assert_eq!(analysis.hashes.get_by_left(&2), Some(&1));
        assert_eq!(analysis.current_node_id(), Some(1));
    }

    #[test]
    fn legacy_tree_is_normalized_with_start_root() {
        let mut tree = Tree::new(Some("analysis"));
        let root_id = tree
            .add_node(Node::new(0, Some(tree_node(1))), None)
            .unwrap();
        tree.add_node(Node::new(1, Some(tree_node(2))), Some(&root_id))
            .unwrap();

        let mut analysis = AnalysisTree {
            current_node: tree.get_node_by_id(&1),
            tree,
            hashes: BiMap::new(),
            game_type: GameType::Base,
            annotations: HashMap::new(),
        };
        analysis.hashes.insert(10, 0);
        analysis.hashes.insert(20, 1);

        analysis.ensure_start_node();

        let start = analysis.tree.get_root_node().unwrap();
        assert_eq!(start.get_node_id().unwrap(), START_NODE_ID);
        assert!(start.get_value().unwrap().is_none());
        assert_eq!(start.get_children_ids().unwrap(), vec![0]);
        assert_eq!(
            analysis
                .tree
                .get_node_by_id(&0)
                .and_then(|node| node.get_parent_id().ok().flatten()),
            Some(START_NODE_ID)
        );
        assert_eq!(
            analysis
                .tree
                .get_node_by_id(&1)
                .and_then(|node| node.get_parent_id().ok().flatten()),
            Some(0)
        );
        assert_eq!(analysis.current_node_id(), Some(1));
        assert_eq!(analysis.hashes.get_by_left(&10), Some(&0));
        assert_eq!(analysis.hashes.get_by_left(&20), Some(&1));
    }
}
