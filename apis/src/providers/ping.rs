use chrono::{DateTime, Duration, Utc};
use leptos::*;

#[derive(Clone, Debug, Copy)]
pub struct PingSignal {
    pub signal: RwSignal<PingState>,
}

impl Default for PingSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl PingSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(PingState::new()),
        }
    }

    pub fn update_ping(&mut self, sent: DateTime<Utc>) {
        self.signal.update(|s| {
            let now = Utc::now();
            s.ping_duration = now.signed_duration_since(sent);
            s.last_update = now;
        })
    }
}

#[derive(Clone, Debug)]
pub struct PingState {
    pub last_update: DateTime<Utc>,
    pub ping_duration: Duration,
}

impl PingState {
    pub fn new() -> Self {
        Self {
            last_update: Utc::now(),
            ping_duration: Duration::seconds(0),
        }
    }
}

impl Default for PingState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_ping() {
    provide_context(PingSignal::new())
}
