use crate::{
    providers::{
        annotations::AnnotationSet,
        game_state::{GameState, GameStateStore, GameStateStoreFields},
    },
    responses::GameResponse,
};
use bimap::BiMap;
use hive_lib::{Color, GameError, GameType, History, State};
use leptos::{prelude::*, reactive::effect::batch};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, vec};
use tree_ds::prelude::{Node, TraversalStrategy, Tree};

const START_NODE_ID: i32 = -1;

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}

#[derive(Clone, Copy)]
pub struct AnalysisSignal {
    pub tree: RwSignal<AnalysisTree>,
    pub sync_reserve: Callback<Color>,
    pub hold_reserve_sync: Callback<()>,
    pub sync_reserve_later: Callback<Color>,
}

impl AnalysisSignal {
    pub fn new(
        tree: AnalysisTree,
        sync_reserve: Callback<Color>,
        hold_reserve_sync: Callback<()>,
        sync_reserve_later: Callback<Color>,
    ) -> Self {
        Self {
            tree: RwSignal::new(tree),
            sync_reserve,
            hold_reserve_sync,
            sync_reserve_later,
        }
    }

    pub fn sync_reserve_from_game_state(&self, game_state: GameStateStore) {
        self.sync_reserve.run(turn_color(game_state));
    }

    pub fn sync_reserve_later_from_game_state(&self, game_state: GameStateStore) {
        self.sync_reserve_later.run(turn_color(game_state));
    }
}

fn turn_color(game_state: GameStateStore) -> Color {
    game_state.state().with_untracked(|state| state.turn_color)
}

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

    pub fn new_blank_analysis(game_state: GameStateStore, game_type: GameType) -> Self {
        game_state.reset_with_game_type(game_type);

        Self::new_with_game_type(game_type)
    }

    pub fn from_loaded_state(state: &State) -> Self {
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
        game_state: GameStateStore,
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
        let selected_state = analysis_tree
            .select_node(target_move_id)
            .or_else(|| analysis_tree.select_node(START_NODE_ID))?;
        let mut next_game_state = GameState::new_with_game_type(selected_state.game_type);
        next_game_state.game_id = Some(game_response.game_id.clone());
        next_game_state.state = selected_state;
        next_game_state.black_id = Some(game_response.black_player.uid);
        next_game_state.white_id = Some(game_response.white_player.uid);
        next_game_state.game_response = Some(game_response.clone());
        game_state.replace(next_game_state);
        Some(analysis_tree)
    }

    pub fn from_uhp(
        game_state: GameStateStore,
        uhp_string: impl Into<String>,
    ) -> Result<Self, GameError> {
        let normalized_uhp = normalize_uhp_metadata(uhp_string);
        let history = match History::from_uhp_str(normalized_uhp) {
            Ok(history) => history,
            Err(GameError::PartialHistory { history, .. }) => history,
            Err(err) => return Err(err),
        };
        let state = State::new_from_history(&history)?;
        let tree = Self::from_loaded_state(&state);
        game_state.reset_with_state(state);
        Ok(tree)
    }

    fn select_node(&mut self, node_id: i32) -> Option<State> {
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

        Some(state)
    }

    pub fn update_node(&mut self, node_id: i32, game: Option<GameStateStore>) -> Option<()> {
        let state = self.select_node(node_id)?;

        if let Some(g) = game {
            batch(|| {
                g.state().set(state);
                g.move_info().update(|move_info| move_info.reset());
            });
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

    pub fn reset(&mut self, game_state: GameStateStore) {
        self.tree = Self::tree_with_start();
        self.current_node = self.tree.get_node_by_id(&START_NODE_ID);
        self.hashes.clear();
        self.annotations.clear();
        game_state.reset_with_game_type(self.game_type);
    }

    // Navigate to the empty board without clearing the analysis tree so the
    // user can go forward again after pressing back past the first move.
    pub fn go_to_start(&mut self, game_state: GameStateStore) {
        self.ensure_start_node();
        self.current_node = self.tree.get_node_by_id(&START_NODE_ID);
        let empty_state = State::new(self.game_type, false);
        batch(|| {
            game_state.state().set(empty_state);
            game_state.move_info().update(|move_info| move_info.reset());
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
    use crate::responses::UserResponse;
    use chrono::Utc;
    use hive_lib::GameStatus;
    use leptos::prelude::Owner;
    use shared_types::{
        Conclusion,
        GameId,
        GameSpeed,
        GameStart,
        Takeback,
        TimeMode,
        TournamentGameResult,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    fn tree_node(turn: usize) -> TreeNode {
        TreeNode {
            turn,
            piece: format!("wA{turn}"),
            position: String::new(),
        }
    }

    fn game_response_from_history(history: History) -> GameResponse {
        let state = State::new_from_history(&history).expect("valid game history");
        let player = |username: &str| UserResponse {
            username: username.to_string(),
            uid: Uuid::new_v4(),
            patreon: false,
            bot: false,
            admin: false,
            deleted: false,
            ratings: HashMap::new(),
            takeback: Takeback::Always,
            lang: None,
        };
        let white_player = player("white");
        let black_player = player("black");
        let current_player_id = match state.turn_color {
            Color::White => white_player.uid,
            Color::Black => black_player.uid,
        };
        let game_status = state.game_status.clone();
        let finished = matches!(
            &game_status,
            GameStatus::Finished(_) | GameStatus::Adjudicated
        );
        let now = Utc::now();

        GameResponse {
            uuid: Uuid::new_v4(),
            game_id: GameId("analysis-game".to_string()),
            tournament: None,
            current_player_id,
            turn: state.turn,
            finished,
            game_status,
            game_type: state.game_type,
            tournament_queen_rule: state.tournament,
            white_player,
            black_player,
            moves: HashMap::new(),
            spawns: Vec::new(),
            rated: false,
            reserve_black: HashMap::new(),
            reserve_white: HashMap::new(),
            history: state.history.moves,
            game_control_history: Vec::new(),
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            time_mode: TimeMode::Untimed,
            time_base: None,
            time_increment: None,
            speed: GameSpeed::Untimed,
            black_time_left: None,
            white_time_left: None,
            last_interaction: None,
            created_at: now,
            updated_at: now,
            hashes: state.history.hashes,
            conclusion: Conclusion::Unknown,
            repetitions: Vec::new(),
            game_start: GameStart::Immediate,
            game_speed: GameSpeed::Untimed,
            move_times: Vec::new(),
            tournament_game_result: TournamentGameResult::Unknown,
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

    #[test]
    fn game_response_middle_move_keeps_tree_and_shared_state_in_sync() {
        let owner = Owner::new();
        owner.with(|| {
            let history = History::from_pgn_str(
                include_str!("../../../engine/test_pgns/valid/descend.pgn").to_string(),
            )
            .expect("valid game history");
            let response = game_response_from_history(history);
            let game_state = GameStateStore::new();
            let selected_node_index = 9;
            let expected_turn = 10;

            let analysis =
                AnalysisTree::from_game_response(&response, game_state, Some(selected_node_index))
                    .expect("analysis tree from game response");

            assert_eq!(analysis.current_node_id(), Some(selected_node_index as i32));
            game_state.state().with_untracked(|selected_state| {
                assert_eq!(selected_state.turn, expected_turn);
                assert_eq!(
                    selected_state.history.moves.as_slice(),
                    &response.history[..expected_turn]
                );
                assert_eq!(
                    selected_state.history.moves.last(),
                    response.history.get(selected_node_index)
                );
            });
        });
    }
}
