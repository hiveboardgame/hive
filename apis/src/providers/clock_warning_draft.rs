use leptos::prelude::*;
use shared_types::TimeWarning;

// Lives above the route because `/config` sits behind a ProtectedRoute that
// remounts whenever auth identity blinks, which would otherwise discard an open
// editor and any unsaved edits. See docs/shitlist.md.
#[derive(Clone, Copy)]
pub struct ClockWarningDraft {
    pub stages: RwSignal<Vec<TimeWarning>>,
    pub editing: RwSignal<Option<usize>>,
    pub seeded: RwSignal<bool>,
}

pub fn provide_clock_warning_draft() {
    provide_context(ClockWarningDraft {
        stages: RwSignal::new(Vec::new()),
        editing: RwSignal::new(None),
        seeded: RwSignal::new(false),
    });
}
