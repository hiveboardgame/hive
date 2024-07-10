use crate::providers::game_state::{GameState, GameStateSignal};
use hive_lib::{GameType, History, State};
use leptos::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, vec};
use tree_ds::prelude::{Node, Tree};

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}

#[derive(Clone)]
pub struct AnalysisSignal(pub RwSignal<Option<AnalysisTree>>);

#[derive(Clone)]
pub struct ToggleStates(pub RwSignal<HashSet<i32>>);

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AnalysisTree {
    pub current_node: Option<Node<i32, TreeNode>>,
    pub tree: Tree<i32, TreeNode>,
}

impl AnalysisTree {
    pub fn from_state(game_state: GameStateSignal) -> Option<Self> {
        let gs = game_state.signal.get_untracked();
        let mut tree = Tree::new(Some("analysis"));
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
            tree.add_node(new_node, previous.as_ref()).ok()?;
            previous = Some(new_id);
        }
        let current_node = previous.and_then(|p| tree.get_node_by_id(&p));
        Some(Self { current_node, tree })
    }

    pub fn update_node(&mut self, node_id: i32) -> Option<()> {
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
            game_type: GameType::MLP,
            ..History::new()
        })
        .ok()?;

        let history_turn = self
            .current_node
            .as_ref()
            .and_then(|n| n.get_value().map(|v| v.turn));

        expect_context::<GameStateSignal>().signal.update(|gs| {
            gs.state = state;
            gs.history_turn = history_turn;
            gs.move_info.reset();
        });
        self.current_node
            .clone_from(&self.tree.get_node_by_id(&node_id));
        Some(())
    }

    pub fn add_node(&mut self, last_move: (String, String)) {
        let (piece, position) = last_move;
        let turn = self
            .current_node
            .as_ref()
            .map_or(1, |n| 1 + n.get_value().unwrap().turn);
        let new_id = self.tree.get_nodes().len() as i32;
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
        self.current_node = self.tree.get_node_by_id(&new_id);
    }

    pub fn reset(&mut self) {
        self.current_node = None;
        self.tree = Tree::new(Some("analysis"));
        let game_state = expect_context::<GameStateSignal>();
        game_state.signal.update(|gs| {
            *gs = GameState::new();
        });
    }
}
