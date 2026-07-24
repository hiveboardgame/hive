use super::{
    document::{
        parse_uhp,
        requested_raw_ply,
        wire_nodes,
        AnalysisDocument,
        LoadError,
        LoadedAnalysis,
        ANALYSIS_FORMAT,
        ANALYSIS_VERSION,
    },
    tree::{AnalysisArena, ChildMatch, MoveDelta, NodeId, PositionCheckpoint, CHECKPOINT_STRIDE},
    view::{build_visible_rows, variation_is_forced_open, BranchSummary, VisibleRow},
};
use crate::{
    providers::{
        annotations::AnnotationSet,
        game_state::{GameState, GameStateStore, GameStateStoreFields},
    },
    responses::GameResponse,
};
use hive_lib::{GameType, History};
use leptos::{prelude::*, reactive::effect::batch};
use reactive_stores::Store;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

#[derive(Clone, Debug, Store)]
pub(super) struct AnalysisState {
    pub(super) arena: AnalysisArena,
    pub(super) checkpoints: HashMap<NodeId, PositionCheckpoint>,
    pub(super) selected_path: Vec<NodeId>,
    pub(super) collapsed: HashSet<NodeId>,
    pub(super) visible_rows: Vec<VisibleRow>,
    pub(super) game_type: GameType,
    pub(super) annotations: HashMap<NodeId, AnnotationSet>,
    pub(super) document_generation: u64,
}

impl AnalysisState {
    pub(super) fn blank(game_type: GameType) -> Self {
        let arena = AnalysisArena::blank();
        Self {
            visible_rows: Vec::new(),
            arena,
            checkpoints: HashMap::new(),
            selected_path: vec![NodeId::ROOT],
            collapsed: HashSet::new(),
            game_type,
            annotations: HashMap::new(),
            document_generation: 0,
        }
    }

    pub(super) fn rebuild_visible_rows(&mut self) {
        self.visible_rows = build_visible_rows(&self.arena, &self.collapsed, &self.selected_path);
    }
}

pub(super) fn selected_node_from_path(path: &[NodeId]) -> NodeId {
    path.last()
        .copied()
        .expect("analysis selected path always contains its root")
}

#[derive(Clone, Copy, Debug)]
pub struct AnalysisStore(pub(super) Store<AnalysisState>);

impl AnalysisStore {
    pub(super) fn new(state: AnalysisState) -> Self {
        Self(Store::new(state))
    }

    pub fn new_blank(game_state: GameStateStore, game_type: GameType) -> Self {
        game_state.reset_with_game_type(game_type);
        Self::new(AnalysisState::blank(game_type))
    }

    pub fn load_game_response(
        &self,
        game_state: GameStateStore,
        game_response: &GameResponse,
        raw_ply: Option<usize>,
    ) -> Result<(), LoadError> {
        let loaded = LoadedAnalysis::from_moves(
            game_response.game_type,
            &game_response.history,
            &game_response.hashes,
            requested_raw_ply(raw_ply, game_response.history.len()),
        )?;
        let mut next_game_state = GameState::new_with_game_type(loaded.playable.game_type);
        next_game_state.game_id = Some(game_response.game_id.clone());
        next_game_state.state = loaded.playable;
        next_game_state.black_id = Some(game_response.black_player.uid);
        next_game_state.white_id = Some(game_response.white_player.uid);
        next_game_state.game_response = Some(game_response.clone());
        self.install_state(game_state, loaded.state, next_game_state);
        Ok(())
    }

    pub fn load_uhp(
        &self,
        game_state: GameStateStore,
        uhp_string: &str,
        raw_ply: Option<usize>,
    ) -> Result<(), LoadError> {
        let history = parse_uhp(uhp_string)?;
        let loaded = LoadedAnalysis::from_partial_moves(
            history.game_type,
            &history.moves,
            &history.hashes,
            requested_raw_ply(raw_ply, history.moves.len()),
        )?;
        self.install_loaded(game_state, loaded);
        Ok(())
    }

    pub fn load_json(&self, game_state: GameStateStore, input: &str) -> Result<(), LoadError> {
        let loaded = LoadedAnalysis::from_json(input)?;
        self.install_loaded(game_state, loaded);
        Ok(())
    }

    pub fn load_pgn(&self, game_state: GameStateStore, input: &str) -> Result<(), LoadError> {
        let history =
            History::from_pgn_str(input).map_err(|error| LoadError::Move(error.to_string()))?;
        let loaded = LoadedAnalysis::from_moves(
            history.game_type,
            &history.moves,
            &history.hashes,
            history.moves.len(),
        )?;
        self.install_loaded(game_state, loaded);
        Ok(())
    }

    fn install_loaded(&self, game_state: GameStateStore, loaded: LoadedAnalysis) {
        let mut next_game_state = GameState::new_with_game_type(loaded.playable.game_type);
        next_game_state.state = loaded.playable;
        self.install_state(game_state, loaded.state, next_game_state);
    }

    fn install_state(
        &self,
        game_state: GameStateStore,
        mut state: AnalysisState,
        next_game_state: GameState,
    ) {
        state.document_generation = self.0.document_generation().get_untracked() + 1;
        batch(|| {
            self.0.set(state);
            game_state.replace(next_game_state);
        });
    }

    pub fn to_json(&self) -> Result<String, LoadError> {
        let (root_id, nodes) = self
            .0
            .arena()
            .with_untracked(|arena| (arena.root, wire_nodes(arena)));
        let document = AnalysisDocument {
            format: ANALYSIS_FORMAT.to_string(),
            version: ANALYSIS_VERSION,
            game_type: self.0.game_type().get_untracked(),
            root_id,
            selected_node_id: self.selected_node_id_untracked(),
            nodes,
            annotations: self.0.annotations().get_untracked(),
        };
        serde_json::to_string(&document).map_err(|error| LoadError::Json(error.to_string()))
    }

    pub fn selected_node_id(&self) -> NodeId {
        self.0
            .selected_path()
            .with(|path| selected_node_from_path(path))
    }

    pub fn selected_node_id_untracked(&self) -> NodeId {
        self.0
            .selected_path()
            .with_untracked(|path| selected_node_from_path(path))
    }

    pub fn document_generation(&self) -> u64 {
        self.0.document_generation().get()
    }

    pub fn game_type_untracked(&self) -> GameType {
        self.0.game_type().get_untracked()
    }

    pub fn is_at_start(&self) -> bool {
        let selected = self.selected_node_id();
        self.0
            .arena()
            .with_untracked(|arena| selected == arena.root)
    }

    pub fn has_moves(&self) -> bool {
        self.0.arena().with(|arena| arena.nodes.len() > 1)
    }

    pub fn selected_hash(&self) -> Option<u64> {
        let selected = self.selected_node_id();
        self.0
            .arena()
            .with(|arena| arena.node(selected).and_then(|node| node.hash))
    }

    pub fn visible_row_count(&self) -> usize {
        self.0.visible_rows().with(Vec::len)
    }

    pub fn visible_row_index(&self, node_id: NodeId) -> Option<usize> {
        self.0
            .visible_rows()
            .with(|rows| rows.iter().position(|row| row.node_id == node_id))
    }

    pub fn visible_rows_in(&self, range: Range<usize>) -> Vec<VisibleRow> {
        self.0.visible_rows().with(|rows| {
            let start = range.start.min(rows.len());
            let end = range.end.min(rows.len()).max(start);
            rows[start..end].to_vec()
        })
    }

    pub fn variations_open(&self, node_id: NodeId) -> bool {
        if !self
            .0
            .collapsed()
            .with(|collapsed| collapsed.contains(&node_id))
        {
            return true;
        }
        self.0.selected_path().with(|selected_path| {
            self.0
                .arena()
                .with(|arena| variation_is_forced_open(arena, node_id, selected_path))
        })
    }

    pub fn node_value_untracked(&self, node_id: NodeId) -> Option<MoveDelta> {
        self.0
            .arena()
            .with_untracked(|arena| arena.node(node_id).and_then(|node| node.value.clone()))
    }

    pub fn node_is_on_current_path(&self, node_id: NodeId) -> bool {
        let Some(depth) = self
            .0
            .arena()
            .with_untracked(|arena| arena.node(node_id).map(|node| node.depth))
        else {
            return false;
        };
        self.0
            .selected_path()
            .with(|path| path.get(depth).copied() == Some(node_id))
    }

    pub fn first_history_target_node_id(&self) -> Option<NodeId> {
        let selected = self.selected_node_id();
        self.0
            .selected_path()
            .with(|path| path.get(1).copied().filter(|target| *target != selected))
    }

    pub fn next_history_target_node_id(&self) -> Option<NodeId> {
        let selected = self.selected_node_id();
        self.0.arena().with(|arena| {
            arena
                .node(selected)
                .and_then(|node| node.children.first().copied())
        })
    }

    pub fn previous_history_target_node_id(&self) -> Option<NodeId> {
        let selected = self.selected_node_id();
        self.0
            .arena()
            .with(|arena| arena.node(selected).and_then(|node| node.parent))
    }

    pub fn select_main_ply(&self, raw_ply: Option<usize>, game_state: GameStateStore) -> bool {
        let target = self.0.arena().with_untracked(|arena| {
            let requested = raw_ply.unwrap_or(usize::MAX);
            let mut current = arena.root;
            let mut depth = 0;
            while depth < requested {
                let Some(next) = arena.node(current)?.children.first().copied() else {
                    break;
                };
                current = next;
                depth += 1;
            }
            Some(current)
        });
        target.is_some_and(|target| self.select_node(target, game_state))
    }

    pub fn select_node(&self, node_id: NodeId, game_state: GameStateStore) -> bool {
        let selected = self.selected_node_id_untracked();
        if node_id == selected {
            game_state.move_info().update(|move_info| move_info.reset());
            return true;
        }
        let adjacent_delta = self.0.arena().with_untracked(|arena| {
            arena.node(node_id).and_then(|node| {
                (node.parent == Some(selected))
                    .then(|| node.value.clone())
                    .flatten()
            })
        });
        if let Some(delta) = adjacent_delta {
            let mut path = self.0.selected_path().get_untracked();
            if path.last().copied() != Some(selected) {
                return false;
            }
            path.push(node_id);
            let rebuild_rows = self.selection_changes_visible_rows(&path);
            let mut played = false;
            batch(|| {
                played = game_state
                    .state()
                    .try_update(|state| {
                        state
                            .play_turn_from_history(&delta.piece, &delta.position)
                            .is_ok()
                    })
                    .unwrap_or(false);
                if played {
                    self.0.selected_path().set(path);
                    game_state.move_info().update(|move_info| move_info.reset());
                }
            });
            if !played {
                return false;
            }
            if rebuild_rows {
                self.rebuild_visible_rows();
            }
            return true;
        }
        let Some((path, state)) = self.0.with_untracked(|analysis| {
            let path = analysis.arena.path_to(node_id)?;
            let state = analysis
                .arena
                .replay(&path, analysis.game_type, &analysis.checkpoints)?;
            Some((path, state))
        }) else {
            return false;
        };
        let rebuild_rows = self.selection_changes_visible_rows(&path);
        batch(|| {
            self.0.selected_path().set(path);
            game_state.state().set(state);
            game_state.move_info().update(|move_info| move_info.reset());
        });
        if rebuild_rows {
            self.rebuild_visible_rows();
        }
        true
    }

    pub fn append_moves(&self, moves: Vec<((String, String), u64)>, game_state: GameStateStore) {
        if moves.is_empty() {
            return;
        }
        let game_type = self.game_type_untracked();
        let mut selected = self.selected_node_id_untracked();
        let starting_depth = self
            .0
            .arena()
            .with_untracked(|arena| arena.node(selected).map(|node| node.depth))
            .unwrap_or_default();
        let append_count = match u64::try_from(moves.len()) {
            Ok(count) => count,
            Err(_) => return,
        };
        let can_append = self.0.arena().with_untracked(|arena| {
            arena.next_id.checked_add(append_count).is_some()
                && arena
                    .node(selected)
                    .and_then(|node| node.depth.checked_add(moves.len()))
                    .is_some()
        });
        if !can_append {
            return;
        }
        let mut traversed = Vec::with_capacity(moves.len());
        let mut new_variation_parent = None;
        let mut reused_canonical_orientation = false;
        self.0.arena().update(|arena| {
            for ((piece, position), hash) in moves {
                let turn = arena.node(selected).map_or(1, |node| node.depth + 1);
                let value = MoveDelta {
                    turn,
                    piece,
                    position,
                };
                if let Some(child_match) = arena.matching_child(selected, &value, hash) {
                    let id = match child_match {
                        ChildMatch::Exact(id) => id,
                        ChildMatch::Canonical(id) => {
                            reused_canonical_orientation = true;
                            id
                        }
                    };
                    selected = id;
                    traversed.push(id);
                    continue;
                }
                let previous_child_count =
                    arena.node(selected).map_or(0, |node| node.children.len());
                let Some(id) = arena.append(selected, value, hash) else {
                    break;
                };
                if previous_child_count == 1 {
                    new_variation_parent = Some(selected);
                }
                selected = id;
                traversed.push(id);
            }
        });
        if traversed.is_empty() {
            return;
        }
        let mut path = self.0.selected_path().get_untracked();
        path.extend(traversed);
        let normalized_state = reused_canonical_orientation.then(|| {
            self.0.with_untracked(|analysis| {
                analysis
                    .arena
                    .replay(&path, game_type, &analysis.checkpoints)
            })
        });
        let normalized_state = match normalized_state {
            Some(Some(state)) => Some(state),
            Some(None) => return,
            None => None,
        };
        let checkpoint = self
            .0
            .arena()
            .with_untracked(|arena| arena.node(selected).map(|node| node.depth))
            .filter(|depth| starting_depth / CHECKPOINT_STRIDE != *depth / CHECKPOINT_STRIDE)
            .map(|_| {
                normalized_state.as_ref().map_or_else(
                    || {
                        game_state
                            .state()
                            .with_untracked(PositionCheckpoint::capture)
                    },
                    PositionCheckpoint::capture,
                )
            });
        batch(|| {
            if let Some(parent) = new_variation_parent {
                self.0.collapsed().update(|collapsed| {
                    collapsed.insert(parent);
                });
            }
            self.0.selected_path().set(path);
            if let Some(state) = normalized_state {
                game_state.state().set(state);
            }
            if let Some(checkpoint) = checkpoint {
                self.0.checkpoints().update(|checkpoints| {
                    checkpoints.insert(selected, checkpoint);
                });
            }
        });
        self.rebuild_visible_rows();
    }

    pub fn reset(&self, game_state: GameStateStore) {
        let game_type = self.game_type_untracked();
        self.reset_with_game_type(game_state, game_type);
    }

    pub fn reset_with_game_type(&self, game_state: GameStateStore, game_type: GameType) {
        let state = AnalysisState::blank(game_type);
        self.install_state(game_state, state, GameState::new_with_game_type(game_type));
    }

    pub fn selected_subtree_summary(&self) -> Option<BranchSummary> {
        let selected = self.selected_node_id_untracked();
        self.0.arena().with_untracked(|arena| {
            let node = arena.node(selected)?;
            if selected == arena.root {
                return None;
            }
            Some(BranchSummary {
                node_id: selected,
                move_delta: node.value.clone()?,
                node_count: arena.subtree_ids(selected).len(),
            })
        })
    }

    pub fn delete_subtree(&self, target: NodeId, game_state: GameStateStore) -> bool {
        let Some(parent) = self.0.arena().with_untracked(|arena| {
            let node = arena.node(target)?;
            (target != arena.root).then_some(node.parent?)
        }) else {
            return false;
        };
        let selected_path = self.0.selected_path().get_untracked();
        let target_index = selected_path.iter().position(|node_id| *node_id == target);
        let replacement = if let Some(target_index) = target_index {
            let path = selected_path[..target_index].to_vec();
            self.0.with_untracked(|analysis| {
                let state =
                    analysis
                        .arena
                        .replay(&path, analysis.game_type, &analysis.checkpoints)?;
                Some((parent, path, state))
            })
        } else {
            None
        };
        if target_index.is_some() && replacement.is_none() {
            return false;
        }

        let mut removed = Vec::new();
        batch(|| {
            self.0
                .arena()
                .update(|arena| removed = arena.remove_subtree(target));
            if removed.is_empty() {
                return;
            }
            self.0.annotations().update(|annotations| {
                for id in &removed {
                    annotations.remove(id);
                }
            });
            self.0.collapsed().update(|collapsed| {
                for id in &removed {
                    collapsed.remove(id);
                }
            });
            self.0.checkpoints().update(|checkpoints| {
                for id in &removed {
                    checkpoints.remove(id);
                }
            });
            if let Some((replacement_id, path, state)) = replacement {
                debug_assert_eq!(path.last().copied(), Some(replacement_id));
                self.0.selected_path().set(path);
                game_state.state().set(state);
                game_state.move_info().update(|move_info| move_info.reset());
            }
            self.rebuild_visible_rows();
        });
        !removed.is_empty()
    }

    pub fn promote_current_variation(&self, all: bool) {
        let path = self.0.selected_path().get_untracked();
        let mut changed = false;
        self.0
            .arena()
            .update(|arena| changed = arena.promote_path(&path, all));
        if changed {
            self.rebuild_visible_rows();
        }
    }

    pub fn toggle_variations(&self, node_id: NodeId) {
        self.0.collapsed().update(|collapsed| {
            if !collapsed.remove(&node_id) {
                collapsed.insert(node_id);
            }
        });
        self.rebuild_visible_rows();
    }

    pub fn alternate_moves(&self) -> Vec<(NodeId, MoveDelta)> {
        let selected = self.selected_node_id();
        self.0.arena().with(|arena| {
            let Some(node) = arena.node(selected) else {
                return Vec::new();
            };
            let Some(parent) = node.parent.and_then(|id| arena.node(id)) else {
                return Vec::new();
            };
            parent
                .children
                .iter()
                .copied()
                .filter(|id| *id != selected)
                .filter_map(|id| arena.node(id)?.value.clone().map(|value| (id, value)))
                .collect()
        })
    }

    pub fn current_annotation(&self) -> AnnotationSet {
        let selected = self.selected_node_id();
        self.0
            .annotations()
            .with(|annotations| annotations.get(&selected).cloned().unwrap_or_default())
    }

    pub fn update_current_annotation(&self, mutate: impl FnOnce(&mut AnnotationSet)) {
        let selected = self.selected_node_id_untracked();
        self.0.annotations().update(|annotations| {
            let mut set = annotations.remove(&selected).unwrap_or_default();
            mutate(&mut set);
            if !set.is_empty() {
                annotations.insert(selected, set);
            }
        });
    }

    fn rebuild_visible_rows(&self) {
        let rows = self.0.with_untracked(|state| {
            build_visible_rows(&state.arena, &state.collapsed, &state.selected_path)
        });
        self.0.visible_rows().set(rows);
    }

    fn selection_changes_visible_rows(&self, next_path: &[NodeId]) -> bool {
        self.0.with_untracked(|state| {
            if state.collapsed.is_empty() {
                return false;
            }
            state.collapsed.iter().any(|node_id| {
                variation_is_forced_open(&state.arena, *node_id, &state.selected_path)
                    != variation_is_forced_open(&state.arena, *node_id, next_path)
            })
        })
    }
}
