use super::{store::AnalysisStore, tree::NodeId};
use crate::providers::game_state::{GameStateStore, GameStateStoreFields};
use hive_lib::{Color, State};
use leptos::{prelude::*, reactive::effect::batch};

/// The real position and selection an opening-explorer preview replaced, so it can be undone.
#[derive(Clone)]
pub struct AnalysisPreviewSnapshot {
    pub node_id: NodeId,
    pub state: State,
    pub generation: u64,
}

#[derive(Clone, Copy)]
pub struct AnalysisContext {
    pub store: AnalysisStore,
    /// Lives here, not in the explorer: a row can unmount without firing `mouseleave`, so every
    /// commit or navigate path must be able to undo an orphaned preview.
    pub preview: RwSignal<Option<AnalysisPreviewSnapshot>>,
    pub sync_reserve: Callback<Color>,
    pub hold_reserve_sync: Callback<()>,
    pub sync_reserve_later: Callback<Color>,
}

impl AnalysisContext {
    pub fn new(
        store: AnalysisStore,
        sync_reserve: Callback<Color>,
        hold_reserve_sync: Callback<()>,
        sync_reserve_later: Callback<Color>,
    ) -> Self {
        Self {
            store,
            preview: RwSignal::new(None),
            sync_reserve,
            hold_reserve_sync,
            sync_reserve_later,
        }
    }

    pub fn reset_preview(&self, game_state: GameStateStore) {
        let Some(snapshot) = self.preview.try_update(Option::take).flatten() else {
            return;
        };
        if self.store.document_generation_untracked() == snapshot.generation
            && self.store.selected_node_id_untracked() == snapshot.node_id
        {
            batch(|| {
                game_state.state().set(snapshot.state);
                game_state.move_info().update(|move_info| move_info.reset());
            });
        }
    }

    /// Undo any preview first, so the store never navigates from a previewed state.
    pub fn select_node(&self, node_id: NodeId, game_state: GameStateStore) -> bool {
        self.reset_preview(game_state);
        self.store.select_node(node_id, game_state)
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
