use leptos::prelude::{provide_context, RwSignal};

use crate::common::GameActionResponse;

#[derive(Clone)]
pub struct GameUpdater {
    pub response: RwSignal<Option<GameActionResponse>>,
}

pub fn provide_game_updater() {
    provide_context(GameUpdater {
        response: RwSignal::new(None),
    });
}
