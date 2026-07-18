use leptos::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct RealtimeAvailability {
    value: RwSignal<Option<bool>>,
    resync_dirty: StoredValue<bool>,
}

impl RealtimeAvailability {
    fn new() -> Self {
        Self {
            value: RwSignal::new(None),
            resync_dirty: StoredValue::new(false),
        }
    }

    pub fn enabled(self) -> bool {
        self.value.get() == Some(true)
    }

    pub fn state(self) -> Option<bool> {
        self.value.get()
    }

    pub fn begin_resync(self) {
        self.resync_dirty.set_value(false);
    }

    pub fn reset_session(self) {
        self.value.set(None);
        self.begin_resync();
    }

    pub fn apply_incremental(self, enabled: bool) {
        self.value.set(Some(enabled));
        self.resync_dirty.set_value(true);
    }

    pub fn apply_snapshot(self, enabled: bool) {
        if !self.resync_dirty.get_value() {
            self.value.set(Some(enabled));
        }
        self.resync_dirty.set_value(false);
    }
}

impl Default for RealtimeAvailability {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_realtime_availability() {
    provide_context(RealtimeAvailability::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_safe_disabled_until_authoritative_state_arrives() {
        let owner = Owner::new();
        owner.set();
        let state = RealtimeAvailability::new();
        assert_eq!(state.state(), None);
        assert!(!state.enabled());
    }

    #[test]
    fn snapshot_applies_without_incremental_update() {
        let owner = Owner::new();
        owner.set();
        let state = RealtimeAvailability::new();
        state.begin_resync();
        state.apply_snapshot(true);
        assert_eq!(state.state(), Some(true));
    }

    #[test]
    fn incremental_update_wins_over_stale_snapshot() {
        let owner = Owner::new();
        owner.set();
        let state = RealtimeAvailability::new();
        state.begin_resync();
        state.apply_incremental(false);
        state.apply_snapshot(true);
        assert_eq!(state.state(), Some(false));
    }

    #[test]
    fn reconnect_resets_to_unknown_and_accepts_new_snapshot() {
        let owner = Owner::new();
        owner.set();
        let state = RealtimeAvailability::new();
        state.apply_incremental(true);
        state.reset_session();
        assert_eq!(state.state(), None);
        assert!(!state.enabled());
        state.apply_snapshot(false);
        assert_eq!(state.state(), Some(false));
    }
}
