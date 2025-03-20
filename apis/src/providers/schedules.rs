use crate::responses::ScheduleResponse;
use leptos::prelude::{provide_context, RwSignal};
use shared_types::GameId;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct SchedulesContext {
    pub own: RwSignal<HashMap<GameId, HashMap<Uuid, ScheduleResponse>>>,
    pub tournament: RwSignal<HashMap<GameId, HashMap<Uuid, ScheduleResponse>>>,
}

pub fn provide_schedules() {
    provide_context(SchedulesContext::default())
}
