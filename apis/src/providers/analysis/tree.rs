use hive_lib::{Board, BoardSnapshot, Color, GameStatus, GameType, History, State};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) const CHECKPOINT_STRIDE: usize = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub(super) u64);

impl NodeId {
    pub(super) const ROOT: Self = Self(0);

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct MoveDelta {
    pub turn: usize,
    pub piece: String,
    pub position: String,
}

#[derive(Clone, Debug)]
pub(super) struct PositionCheckpoint {
    board: BoardSnapshot,
    game_id: u64,
    turn: usize,
    turn_color: Color,
    game_status: GameStatus,
    game_type: GameType,
    tournament: bool,
    current_hash: Option<u64>,
}

impl PositionCheckpoint {
    pub(super) fn capture(state: &State) -> Self {
        Self {
            board: state.board.snapshot(),
            game_id: state.game_id,
            turn: state.turn,
            turn_color: state.turn_color,
            game_status: state.game_status.clone(),
            game_type: state.game_type,
            tournament: state.tournament,
            current_hash: state.hashes.last().copied(),
        }
    }

    fn restore(&self) -> State {
        let mut state = State::new(self.game_type, self.tournament);
        state.game_id = self.game_id;
        state.board = Board::from_snapshot(&self.board);
        state.turn = self.turn;
        state.turn_color = self.turn_color;
        state.game_status = self.game_status.clone();
        state
    }
}

#[derive(Clone, Debug)]
pub(super) struct AnalysisNode {
    pub(super) parent: Option<NodeId>,
    pub(super) children: Vec<NodeId>,
    pub(super) value: Option<MoveDelta>,
    pub(super) hash: Option<u64>,
    pub(super) depth: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChildMatch {
    Exact(NodeId),
    Canonical(NodeId),
}

#[derive(Clone, Debug)]
pub(super) struct AnalysisArena {
    pub(super) root: NodeId,
    pub(super) nodes: HashMap<NodeId, AnalysisNode>,
    pub(super) next_id: u64,
}

impl AnalysisArena {
    pub(super) fn blank() -> Self {
        let root = AnalysisNode {
            parent: None,
            children: Vec::new(),
            value: None,
            hash: None,
            depth: 0,
        };
        Self {
            root: NodeId::ROOT,
            nodes: HashMap::from([(NodeId::ROOT, root)]),
            next_id: 1,
        }
    }

    pub(super) fn node(&self, id: NodeId) -> Option<&AnalysisNode> {
        self.nodes.get(&id)
    }

    pub(super) fn matching_child(
        &self,
        parent: NodeId,
        value: &MoveDelta,
        hash: u64,
    ) -> Option<ChildMatch> {
        let children = &self.node(parent)?.children;
        children
            .iter()
            .copied()
            .find(|id| {
                self.node(*id)
                    .is_some_and(|node| node.value.as_ref() == Some(value))
            })
            .map(ChildMatch::Exact)
            .or_else(|| {
                children
                    .iter()
                    .copied()
                    .find(|id| self.node(*id).is_some_and(|node| node.hash == Some(hash)))
                    .map(ChildMatch::Canonical)
            })
    }

    pub(super) fn path_to(&self, target: NodeId) -> Option<Vec<NodeId>> {
        let mut path = Vec::new();
        let mut current = Some(target);
        while let Some(id) = current {
            let node = self.node(id)?;
            path.push(id);
            current = node.parent;
        }
        if path.last().copied() != Some(self.root) {
            return None;
        }
        path.reverse();
        Some(path)
    }

    pub(super) fn append(&mut self, parent: NodeId, value: MoveDelta, hash: u64) -> Option<NodeId> {
        let depth = self.node(parent)?.depth.checked_add(1)?;
        let id = NodeId(self.next_id);
        let next_id = self.next_id.checked_add(1)?;
        self.nodes.insert(
            id,
            AnalysisNode {
                parent: Some(parent),
                children: Vec::new(),
                value: Some(value),
                hash: Some(hash),
                depth,
            },
        );
        self.nodes.get_mut(&parent)?.children.push(id);
        self.next_id = next_id;
        Some(id)
    }

    pub(super) fn remove_subtree(&mut self, subtree_root: NodeId) -> Vec<NodeId> {
        if subtree_root == self.root || !self.nodes.contains_key(&subtree_root) {
            return Vec::new();
        }
        let parent = self.node(subtree_root).and_then(|node| node.parent);
        let removed = self.subtree_ids(subtree_root);
        if let Some(parent) = parent.and_then(|id| self.nodes.get_mut(&id)) {
            parent.children.retain(|id| *id != subtree_root);
        }
        for id in &removed {
            self.nodes.remove(id);
        }
        removed
    }

    pub(super) fn subtree_ids(&self, subtree_root: NodeId) -> Vec<NodeId> {
        if !self.nodes.contains_key(&subtree_root) {
            return Vec::new();
        }
        let mut ids = Vec::new();
        let mut stack = vec![subtree_root];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.node(id) {
                stack.extend(node.children.iter().copied());
                ids.push(id);
            }
        }
        ids
    }

    pub(super) fn promote_path(&mut self, path: &[NodeId], all: bool) -> bool {
        let mut changed = false;
        for edge in path.windows(2).rev() {
            let [parent_id, child_id] = edge else {
                continue;
            };
            let Some(parent) = self.nodes.get_mut(parent_id) else {
                continue;
            };
            let Some(index) = parent.children.iter().position(|id| id == child_id) else {
                continue;
            };
            if index > 0 {
                let child = parent.children.remove(index);
                parent.children.insert(0, child);
                changed = true;
                if !all {
                    break;
                }
            }
        }
        changed
    }

    pub(super) fn replay(
        &self,
        path: &[NodeId],
        game_type: GameType,
        checkpoints: &HashMap<NodeId, PositionCheckpoint>,
    ) -> Option<State> {
        if path.first().copied() != Some(self.root)
            || path.windows(2).any(|edge| {
                let [parent, child] = edge else {
                    return true;
                };
                self.node(*child)
                    .is_none_or(|node| node.parent != Some(*parent))
            })
        {
            return None;
        }
        let checkpoint = path
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, id)| checkpoints.get(id).map(|checkpoint| (index, checkpoint)));
        let (mut state, replay_start) = checkpoint.map_or_else(
            || (State::new(game_type, false), 1),
            |(index, checkpoint)| (checkpoint.restore(), index + 1),
        );
        state.history.game_type = game_type;
        let context_end = replay_start.saturating_sub(1);
        if context_end > 0 {
            let mut moves = Vec::with_capacity(context_end);
            let mut hashes = Vec::with_capacity(context_end);
            for id in path.iter().take(context_end + 1).skip(1) {
                let node = self.node(*id)?;
                let delta = node.value.as_ref()?;
                moves.push((delta.piece.clone(), delta.position.clone()));
                hashes.push(node.hash?);
            }
            if checkpoint
                .is_some_and(|(_, checkpoint)| checkpoint.current_hash != hashes.last().copied())
            {
                return None;
            }
            state.history = History {
                moves,
                hashes: hashes.clone(),
                game_type,
                ..History::new()
            };
            state.hashes = hashes;
            for hash in &state.hashes {
                *state.hashes_count.entry(*hash).or_default() += 1;
            }
            if state
                .hashes
                .last()
                .and_then(|hash| state.hashes_count.get(hash))
                .is_some_and(|count| *count > 2)
            {
                let repeated_hash = *state.hashes.last()?;
                state.repeating_moves = state
                    .hashes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, hash)| (*hash == repeated_hash).then_some(index))
                    .collect();
            }
        }
        for id in path.iter().copied().skip(replay_start) {
            let delta = self.node(id)?.value.as_ref()?;
            state
                .play_turn_from_history(&delta.piece, &delta.position)
                .ok()?;
        }
        Some(state)
    }
}
