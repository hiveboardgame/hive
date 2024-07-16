use crate::responses::ScheduleResponse;
use leptos::{provide_context, RwSignal};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct SchedulesContext {
    pub own: RwSignal<HashMap<Uuid, Vec<ScheduleResponse>>>,
    pub tournament: RwSignal<Vec<ScheduleResponse>>,
}

pub fn provide_schedules() {
    provide_context(SchedulesContext::default())
}
