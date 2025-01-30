use chrono::{DateTime, Utc};
use leptos::prelude::*;

#[derive(Clone, Debug, Copy)]
pub struct PingContext {
    pub ping: RwSignal<f64>,
    pub last_updated: RwSignal<DateTime<Utc>>,
}

impl PingContext {
    pub fn update_ping(&mut self, ping: f64) {
        self.ping.set(ping);
        self.last_updated.set(Utc::now());
    }
}

impl Default for PingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PingContext {
    pub fn new() -> Self {
        Self {
            ping: 0.0.into(),
            last_updated: Utc::now().into(),
        }
    }
}

pub fn provide_ping() {
    provide_context(PingContext::new())
}
