use leptos::prelude::{provide_context, RwSignal};
use shared_types::{GameId, ReadyUser};
use std::collections::HashMap;

use crate::{common::GameActionResponse, responses::HeartbeatResponse};

#[derive(Clone)]
pub struct UpdateNotifier {
    pub game_response: RwSignal<Option<GameActionResponse>>,
    pub heartbeat: RwSignal<HeartbeatResponse>,
    pub tournament_ready: RwSignal<HashMap<GameId, Vec<ReadyUser>>>,
    // Resource keys must change even when consecutive updates concern the same tournament.
    pub tournament_catalog_update: RwSignal<u64>,
}

pub fn provide_server_updates() {
    provide_context(UpdateNotifier {
        game_response: RwSignal::new(None),
        heartbeat: RwSignal::new(HeartbeatResponse::default()),
        tournament_ready: RwSignal::new(HashMap::new()),
        tournament_catalog_update: RwSignal::new(0),
    });
}
