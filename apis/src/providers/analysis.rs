use crate::providers::game_state::{GameState, GameStateSignal};
use crate::responses::GameResponse;
use bimap::BiMap;
use hive_lib::{GameError, GameType, History, State};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::vec;
use tree_ds::prelude::{Node, Tree};

use super::game_state;
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}
#[derive(Clone)]
pub struct AnalysisSignal(pub RwSignal<AnalysisTree>);

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AnalysisTree {
    pub current_node: Option<Node<i32, TreeNode>>,
    pub tree: Tree<i32, TreeNode>,
    pub hashes: BiMap<u64, i32>,
    pub game_type: GameType,
}

impl AnalysisTree {
    pub fn new_blank_analysis(game_state: GameStateSignal, game_type: GameType) -> Self {
        let tree = Tree::new(Some("analysis"));
        let hashes = BiMap::new();
        game_state.signal.update(|gs| {
            *gs = GameState::new_with_game_type(game_type);
            gs.view = game_state::View::Game;
        });

        Self {
            current_node: None,
            tree,
            hashes,
            game_type,
        }
    }

    pub fn from_loaded_state(game_state: GameStateSignal, state: &State) -> Self {
        let mut tree = Tree::new(Some("analysis"));
        let mut hashes = BiMap::new();
        let mut previous = None;

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
        }
    }

    pub fn from_game_response(
        game_response: &GameResponse,
        game_state: GameStateSignal,
        move_number: Option<usize>,
    ) -> Option<Self> {
        let state = game_response.create_state();
        let mut tree = Tree::new(Some("analysis"));
        let mut hashes = BiMap::new();
        let mut previous = None;

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

        let target_move_id = move_number.unwrap_or(move_count.saturating_sub(1)) as i32;
        analysis_tree.update_node(target_move_id, Some(game_state));
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
        let state = State::new_from_history(&History {
            moves,
            game_type: self.game_type,
            ..History::new()
        })
        .ok()?;

        self.current_node = self.tree.get_node_by_id(&node_id);

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
        let mut new_id = self.tree.get_nodes().len() as i32;
        while self.tree.get_node_by_id(&new_id).is_some() {
            new_id += 1;
        }
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
            .and_then(|n| n.get_node_id().ok());

        if let Some(parent_id) = parent_id {
            let _ = self.tree.add_node(new_node, Some(&parent_id));
        } else {
            let _ = self.tree.add_node(new_node, None);
        }
        self.hashes.insert(hash, new_id);
        self.current_node = self.tree.get_node_by_id(&new_id);
    }

    pub fn reset(&mut self, game_state: GameStateSignal) {
        self.current_node = None;
        self.tree = Tree::new(Some("analysis"));
        self.hashes.clear();
        game_state.signal.update(|gs| {
            *gs = GameState::new_with_game_type(self.game_type);
            gs.view = game_state::View::Game;
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
