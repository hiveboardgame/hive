use chrono::{DateTime, TimeDelta, Utc};
use leptos::prelude::*;
use leptos_use::use_interval_fn;

pub fn use_ticking_now() -> Signal<DateTime<Utc>> {
    let now = RwSignal::new(Utc::now());
    use_interval_fn(move || now.set(Utc::now()), 1000);
    now.into()
}

pub fn format_time_left(left: TimeDelta) -> String {
    let total = left.num_seconds().max(0);
    if total >= 3600 {
        format!("{}h {:02}m", total / 3600, (total % 3600) / 60)
    } else {
        format!("{}:{:02}", total / 60, total % 60)
    }
}
