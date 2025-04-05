use crate::providers::game_state::{GameState, GameStateSignal};
use bimap::BiMap;
use hive_lib::{GameType, History, State};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use serde::{Deserialize, Serialize};
use std::vec;
use tree_ds::prelude::{Node, Tree};
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}
#[derive(Clone)]
pub struct AnalysisSignal(pub RwSignal<SendWrapper<AnalysisTree>>);

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AnalysisTree {
    pub current_node: Option<Node<i32, TreeNode>>,
    pub tree: Tree<i32, TreeNode>,
    pub hashes: BiMap<u64, i32>,
    pub game_type: GameType,
}

impl AnalysisTree {
    pub fn from_state(game_state: GameStateSignal) -> Option<Self> {
        let gs = game_state.signal.get_untracked();
        let mut tree = Tree::new(Some("analysis"));
        let mut hashes = BiMap::new();
        let mut previous = None;
        for (i, (piece, position)) in gs.state.history.moves.iter().enumerate() {
            let new_node = Node::new(
                i as i32,
                Some(TreeNode {
                    turn: i + 1,
                    piece: piece.to_string(),
                    position: position.to_string(),
                }),
            );
            let new_id = new_node.get_node_id();
            let hash = gs.state.history.hashes[i];
            tree.add_node(new_node, previous.as_ref()).ok()?;
            hashes.insert(hash, new_id);
            previous = Some(new_id);
        }
        let current_node = previous.and_then(|p| tree.get_node_by_id(&p));
        let mut tree = Self {
            current_node,
            tree,
            hashes,
            game_type: gs.state.game_type,
        };
        tree.update_node(gs.history_turn.unwrap_or(0) as i32, Some(game_state));
        Some(tree)
    }

    pub fn update_node(&mut self, node_id: i32, game: Option<GameStateSignal>) -> Option<()> {
        let moves = self
            .tree
            .get_ancestor_ids(&node_id)
            .ok()?
            .into_iter()
            .rev()
            .chain(vec![node_id])
            .map(|a| self.tree.get_node_by_id(&a)?.get_value())
            .map(|a| {
                let a = a.unwrap();
                (a.piece, a.position)
            })
            .collect::<Vec<_>>();
        let state = State::new_from_history(&History {
            moves,
            game_type: self.game_type,
            ..History::new()
        })
        .ok()?;

        let history_turn = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_value().map(|v| v.turn));

        if let Some(g) = game {
            g.signal.update(|gs| {
                gs.state = state;
                gs.history_turn = history_turn;
                gs.move_info.reset();
            })
        }
        self.current_node
            .clone_from(&self.tree.get_node_by_id(&node_id));
        Some(())
    }

    pub fn add_node(&mut self, last_move: (String, String), hash: u64) {
        let (piece, position) = last_move;
        let turn = self
            .current_node
            .as_ref()
            .map_or(1, |n| 1 + n.get_value().unwrap().turn);
        let valid_trasposition = self
            .hashes
            .get_by_left(&hash)
            .and_then(|node_id| self.tree.get_node_by_id(node_id))
            .and_then(|node| {
                //Turns must match if we will update the node
                if node.get_value().unwrap().turn == turn {
                    self.update_node(node.get_node_id(), None)
                } else {
                    None
                }
            });
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
        let parent_id = self.current_node.as_ref().map(|n| n.get_node_id());

        self.tree.add_node(new_node, parent_id.as_ref()).unwrap();
        self.hashes.insert(hash, new_id);
        self.current_node = self.tree.get_node_by_id(&new_id);
    }

    pub fn reset(&mut self) {
        self.current_node = None;
        self.tree = Tree::new(Some("analysis"));
        let game_state = expect_context::<GameStateSignal>();
        game_state.signal.update(|gs| {
            *gs = GameState::new_with_game_type(self.game_type);
        });
    }
}
