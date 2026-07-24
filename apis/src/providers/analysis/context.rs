use super::store::AnalysisStore;
use crate::providers::game_state::{GameStateStore, GameStateStoreFields};
use hive_lib::Color;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct AnalysisContext {
    pub store: AnalysisStore,
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
