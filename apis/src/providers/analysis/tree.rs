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
            tournament: state.tournament,
            current_hash: state.hashes.last().copied(),
        }
    }

    fn restore(&self, game_type: GameType) -> State {
        let mut state = State::new(game_type, self.tournament);
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
        let mut canonical = None;
        for id in children.iter().copied() {
            let Some(node) = self.node(id) else {
                continue;
            };
            if node.value.as_ref() == Some(value) {
                return Some(ChildMatch::Exact(id));
            }
            if canonical.is_none() && node.hash == Some(hash) {
                canonical = Some(ChildMatch::Canonical(id));
            }
        }
        canonical
    }

    /// Equal depth and equal hash anywhere in the tree; smallest id for determinism.
    pub(super) fn transposition(&self, hash: u64, depth: usize) -> Option<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.depth == depth && node.hash == Some(hash))
            .map(|(id, _)| *id)
            .min()
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
        if subtree_root == self.root {
            return Vec::new();
        }
        let parent = self.node(subtree_root).and_then(|node| node.parent);
        let removed = self.subtree_ids(subtree_root).collect::<Vec<_>>();
        if let Some(parent) = parent.and_then(|id| self.nodes.get_mut(&id)) {
            parent.children.retain(|id| *id != subtree_root);
        }
        for id in &removed {
            self.nodes.remove(id);
        }
        removed
    }

    pub(super) fn subtree_ids(&self, subtree_root: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let mut stack = vec![subtree_root];
        std::iter::from_fn(move || {
            while let Some(id) = stack.pop() {
                let Some(node) = self.node(id) else {
                    continue;
                };
                stack.extend(node.children.iter().copied());
                return Some(id);
            }
            None
        })
    }

    pub(super) fn promote_path(&mut self, path: &[NodeId], all: bool) -> bool {
        let mut changed = false;
        for edge in path.windows(2).rev() {
            let parent_id = edge[0];
            let child_id = edge[1];
            let Some(parent) = self.nodes.get_mut(&parent_id) else {
                continue;
            };
            let Some(index) = parent.children.iter().position(|id| *id == child_id) else {
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
                self.node(edge[1])
                    .is_none_or(|node| node.parent != Some(edge[0]))
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
            |(index, checkpoint)| (checkpoint.restore(game_type), index + 1),
        );
        // A HOP root has occurred once and checkpoints carry no counts, so count it here. No
        // double count: the loop below skips `path[0]`.
        if let Some(root_hash) = self.node(self.root).and_then(|node| node.hash) {
            *state.hashes_count.entry(root_hash).or_default() += 1;
        }
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
                // Saturating, as in `State::three_fold_repetition`: an analysis line can shuffle
                // a position as often as it likes, and a wrapped count would un-detect it.
                let count = state.hashes_count.entry(*hash).or_default();
                *count = count.saturating_add(1);
            }
        }
        // A path that continued past a repetition must still replay (see `State::replaying`).
        let mut replay_moves = Vec::new();
        for id in path.iter().copied().skip(replay_start) {
            let delta = self.node(id)?.value.as_ref()?;
            replay_moves.push((delta.piece.as_str(), delta.position.as_str()));
        }
        state.replay_turns(replay_moves).ok()?;
        // The checkpointed context was never replayed, so the markers must come from the full
        // hash sequence - otherwise a repetition buried in the checkpoint loses them.
        let mut seen: HashMap<u64, u8> = HashMap::new();
        // A HOP root occurred once but sits in no hash list; without seeding it here, a
        // threefold formed with the root loses its markers on a checkpointed rebuild.
        if let Some(root_hash) = self.node(self.root).and_then(|node| node.hash) {
            seen.insert(root_hash, 1);
        }
        let mut repeated = None;
        for hash in &state.hashes {
            let count = seen.entry(*hash).or_default();
            *count = count.saturating_add(1);
            if *count > 2 {
                repeated = Some(*hash);
            }
        }
        if let Some(repeated) = repeated {
            state.repeating_moves = state
                .hashes
                .iter()
                .enumerate()
                .filter_map(|(index, hash)| (*hash == repeated).then_some(index))
                .collect();
        }
        // A threefold with nothing recorded after it was the game's end, not a grandfathered
        // continuation - reaching it by navigation must show the draw.
        let target_is_leaf = path
            .last()
            .and_then(|id| self.node(*id))
            .is_none_or(|node| node.children.is_empty());
        if target_is_leaf {
            state.finish_repetition_at_final_ply();
        }
        Some(state)
    }
}
