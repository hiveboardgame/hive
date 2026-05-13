use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_use::use_interval_fn;

pub const FRESH_WINDOW_SECS: i64 = 5;

#[derive(Clone, Debug, Copy)]
pub struct PingContext {
    pub ping: RwSignal<f64>,
    pub last_updated: RwSignal<DateTime<Utc>>,
    /// True iff a ping arrived within the last `FRESH_WINDOW_SECS` seconds.
    /// Reactive on both `last_updated` and a 1Hz wall-clock tick, so it
    /// updates the moment a fresh ping arrives *and* the moment the
    /// window lapses with no new ping.
    pub is_fresh: Signal<bool>,
}

impl PingContext {
    pub fn update_ping(&mut self, ping: f64) {
        self.ping.set(ping);
        self.last_updated.set(Utc::now());
    }

    /// Bump `last_updated` without changing the round-trip value. Used by the
    /// zombie-socket detector so that a freshly forced reconnect gets a
    /// `FRESH_WINDOW_SECS` grace period before the staleness check fires
    /// again — an in-flight reconnect that hasn't yet received its first
    /// server ping must not immediately re-trigger another reopen.
    pub fn mark_active(&self) {
        self.last_updated.set(Utc::now());
    }
}

pub fn provide_ping() {
    let ping = RwSignal::new(0.0);
    let last_updated = RwSignal::new(Utc::now());

    // Reactive "now" — advanced by an interval so derived freshness
    // checks invalidate as time passes. On SSR `use_interval_fn` is a
    // no-op, which is fine; the initial value is good enough for the
    // server-rendered frame.
    let now = RwSignal::new(Utc::now());
    use_interval_fn(move || now.set(Utc::now()), 1000);

    let is_fresh = Memo::new(move |_| {
        now.get()
            .signed_duration_since(last_updated.get())
            .num_seconds()
            < FRESH_WINDOW_SECS
    });

    provide_context(PingContext {
        ping,
        last_updated,
        is_fresh: is_fresh.into(),
    });
}
