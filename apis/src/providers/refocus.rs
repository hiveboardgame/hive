use chrono::{DateTime, Utc};
use leptos::*;


#[derive(Clone, Debug, Copy)]
pub struct RefocusSignal {
    pub signal: RwSignal<RefocusState>,
}

impl Default for RefocusSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl RefocusSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(RefocusState::new()),
        }
    }

    pub fn refocus(&mut self) {
        //let now = Utc::now();
        //let time_away = now.signed_duration_since(self.signal.get().defocus_at);
        //log!("Was away for: {}", time_away);
        //let ping = expect_context::<PingSignal>();
        //let time_last_ping = now.signed_duration_since(ping.signal.get().last_update);
        //log!(
        //    "Was away for: {} last_ping was {}s ago",
        //    time_away,
        //    time_last_ping
        //);
        self.signal.update(|s| s.focused = true);
    }

    pub fn unfocus(&mut self) {
        self.signal.update(|s| {
            s.defocus_at = Utc::now();
            s.focused = false;
        })
    }
}

#[derive(Clone, Debug)]
pub struct RefocusState {
    pub focused: bool,
    pub defocus_at: DateTime<Utc>,
}

impl RefocusState {
    pub fn new() -> Self {
        Self {
            focused: true,
            defocus_at: Utc::now(),
        }
    }
}

impl Default for RefocusState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_refocus() {
    provide_context(RefocusSignal::new())
}
