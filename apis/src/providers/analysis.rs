use crate::{
    providers::game_state::{GameState, GameStateStore, GameStateStoreFields},
    responses::GameResponse,
};
use bimap::BiMap;
use hive_lib::{GameType, History, State};
use leptos::prelude::*;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, vec};
use tree_ds::prelude::{Node, Tree};

use super::game_state;
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}
#[derive(Clone, Copy)]
pub struct AnalysisStore(pub Store<AnalysisTree>);

impl Deref for AnalysisStore {
    type Target = Store<AnalysisTree>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AnalysisStore {
    pub fn update_node(&self, node_id: i32) {
        self.current_node()
            .set(self.tree().get().get_node_by_id(&node_id));
    }

    pub fn sync_game_state(&self, game: GameStateStore) -> Option<()> {
        let mut success = true;
        let current_node_id = self.current_node().get().and_then(|n| n.get_node_id().ok());
        if let Some(current_id) = current_node_id {
            if let Some(path_pos) = self.full_path().get().iter().position(|n| n == &current_id) {
                let history_turn = game.history_turn().get().unwrap_or_default();
                let path_pos = path_pos + 1;
                if path_pos <= history_turn {
                    game.undo_n_moves(history_turn - path_pos);
                } else {
                    let moves_to_play = self
                        .full_path()
                        .get()
                        .iter()
                        .skip(history_turn)
                        .take(path_pos - history_turn)
                        .filter_map(|node_id| {
                            self.tree()
                                .get()
                                .get_node_by_id(node_id)
                                .and_then(|node| node.get_value().ok())
                                .flatten()
                                .map(|tree_node| (tree_node.piece, tree_node.position))
                        })
                        .collect::<Vec<_>>();
                    game.state().update(|state| {
                        for (piece, position) in moves_to_play {
                            if let Err(e) = state.play_turn_from_history(&piece, &position) {
                                leptos::logging::log!(
                                    "Could not play history turn: {} {} {}",
                                    piece,
                                    position,
                                    e
                                );
                                break;
                            }
                        }
                    });
                    game.history_turn().set(Some(path_pos));
                }
            } else if let Some((state, history_turn)) = self.with(|a| a.state_and_turn_for_node()) {
                game.state().set(state);
                game.history_turn().set(history_turn);
                game.move_info().update(|m| m.reset());
                self.update(|a| a.recompute_full_path_from_current());
            } else {
                success = false;
            }
        } else {
            game.reset();
        }
        success.then_some(())
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Store)]
pub struct AnalysisTree {
    pub current_node: Option<Node<i32, TreeNode>>,
    pub tree: Tree<i32, TreeNode>,
    pub hashes: BiMap<u64, i32>,
    pub game_type: GameType,
    #[serde(skip)]
    pub full_path: Vec<i32>,
}

impl AnalysisTree {
    fn update_node_if_in_range(&mut self, move_number: Option<usize>) {
        let Some(move_idx) = move_number else {
            return;
        };
        let node_id = move_idx as i32;
        if self.tree.get_node_by_id(&node_id).is_some() {
            self.update_node(node_id);
        }
    }

    pub fn new_blank_analysis(game_type: GameType) -> Self {
        let tree = Tree::new(Some("analysis"));
        let hashes = BiMap::new();
        Self {
            current_node: None,
            tree,
            hashes,
            game_type,
            full_path: vec![],
        }
    }

    pub fn from_loaded_state(state: &State, move_number: Option<usize>) -> Self {
        let mut tree = Tree::new(Some("analysis"));
        let mut hashes = BiMap::new();
        let mut previous = None;
        let mut full_path = Vec::new();
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
                full_path.push(new_id);
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
            full_path,
        };
        analysis_tree.update_node_if_in_range(move_number);
        analysis_tree
    }

    pub fn from_game_response(game_response: &GameResponse, move_number: Option<usize>) -> Self {
        let state = game_response.create_state();
        Self::from_loaded_state(&state, move_number)
    }

    pub fn update_node(&mut self, node_id: i32) {
        self.current_node = self.tree.get_node_by_id(&node_id);
    }

    pub fn recompute_full_path_from_current(&mut self) {
        let Some(current_id) = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_node_id().ok())
        else {
            self.full_path.clear();
            return;
        };

        let mut full_path = self
            .tree
            .get_ancestor_ids(&current_id)
            .ok()
            .map(|ids| ids.into_iter().rev().collect::<Vec<_>>())
            .unwrap_or_default();
        full_path.push(current_id);

        let mut cursor = current_id;
        while let Some(next_id) = self
            .tree
            .get_node_by_id(&cursor)
            .and_then(|node| node.get_children_ids().ok())
            .and_then(|children| children.first().copied())
        {
            full_path.push(next_id);
            cursor = next_id;
        }
        self.full_path = full_path;
    }

    pub fn state_and_turn_for_node(&self) -> Option<(State, Option<usize>)> {
        let node_id = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_node_id().ok())?;
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
        let history_turn = self
            .tree
            .get_node_by_id(&node_id)
            .as_ref()
            .and_then(|n| n.get_value().ok())
            .flatten()
            .map(|v| v.turn);
        Some((state, history_turn))
    }

    pub fn add_node(&mut self, last_move: (String, String), hash: u64) {
        let (piece, position) = last_move;
        let previous_node_id = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_node_id().ok());
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
                    (Some(v), Some(node_id)) if v.turn == turn => Some(node_id),
                    _ => None,
                },
            );
        if let Some(node_id) = valid_trasposition {
            if let Some(previous_node_id) = previous_node_id {
                while self
                    .full_path
                    .last()
                    .is_some_and(|id| *id != previous_node_id)
                {
                    self.full_path.pop();
                }
            } else {
                self.full_path.clear();
            }
            self.full_path.push(node_id);
            self.current_node = self.tree.get_node_by_id(&node_id);
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
        let _ = self.tree.add_node(new_node, parent_id.as_ref());
        self.hashes.insert(hash, new_id);
        self.current_node = self.tree.get_node_by_id(&new_id);
        if let Some(previous_node_id) = previous_node_id {
            while self
                .full_path
                .last()
                .is_some_and(|id| *id != previous_node_id)
            {
                self.full_path.pop();
            }
        } else {
            self.full_path.clear();
        }
        self.full_path.push(new_id);
    }

    pub fn reset(&mut self, game_state: GameStateStore) {
        self.current_node = None;
        self.tree = Tree::new(Some("analysis"));
        self.hashes.clear();
        self.full_path.clear();
        game_state.update(|gs| {
            *gs = GameState::new_with_game_type(self.game_type);
            gs.view = game_state::View::Game;
        });
    }
}
