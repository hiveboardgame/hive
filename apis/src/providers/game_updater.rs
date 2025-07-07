use leptos::prelude::{provide_context, RwSignal};
use shared_types::{GameId, TournamentId};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{common::GameActionResponse, responses::HeartbeatResponse};

#[derive(Clone)]
pub struct UpdateNotifier {
    pub game_response: RwSignal<Option<GameActionResponse>>,
    pub heartbeat: RwSignal<HeartbeatResponse>,
    pub tournament_ready: RwSignal<HashMap<GameId, Vec<(Uuid, String)>>>,
    pub tournament_update: RwSignal<TournamentId>,
}

pub fn provide_server_updates() {
    provide_context(UpdateNotifier {
        game_response: RwSignal::new(None),
        heartbeat: RwSignal::new(HeartbeatResponse::default()),
        tournament_ready: RwSignal::new(HashMap::new()),
        tournament_update: RwSignal::new(TournamentId::default()),
    });
}
