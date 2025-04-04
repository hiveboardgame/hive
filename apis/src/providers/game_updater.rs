use leptos::prelude::{provide_context, RwSignal};

use crate::{common::GameActionResponse, responses::HeartbeatResponse};


#[derive(Clone)]
pub struct GameUpdater {
    pub response: RwSignal<Option<GameActionResponse>>,
    pub heartbeat: RwSignal<Option<HeartbeatResponse>>,
}

pub fn provide_game_updater() {
    provide_context(GameUpdater {
        response: RwSignal::new(None),
        heartbeat: RwSignal::new(None),
    });
}
