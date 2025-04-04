use leptos::prelude::{provide_context, RwSignal};
use shared_types::GameId;
use uuid::Uuid;

use crate::{common::GameActionResponse, responses::HeartbeatResponse};

#[derive(Clone)]
pub struct GameUpdater {
    pub response: RwSignal<Option<GameActionResponse>>,
    pub heartbeat: RwSignal<HeartbeatResponse>,
    pub tournament_ready: RwSignal<(GameId, Uuid)>,
}

pub fn provide_game_updater() {
    provide_context(GameUpdater {
        response: RwSignal::new(None),
        heartbeat: RwSignal::new(HeartbeatResponse::default()),
        tournament_ready: RwSignal::new((GameId::default(), Uuid::default())),
    });
}
