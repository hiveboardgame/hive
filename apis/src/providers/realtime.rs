use leptos::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct RealtimeEnabledContext(pub RwSignal<bool>);

pub fn provide_realtime_enabled() {
    provide_context(RealtimeEnabledContext(RwSignal::new(true)));
}
