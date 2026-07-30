use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_use::use_interval_fn;

/// A reactive "now" that advances once a second.
///
/// An arena ends on a wall clock rather than a round count, so anything showing
/// how long is left has to re-derive it as time passes — otherwise the figure is
/// frozen at whenever the tournament response last arrived. On SSR
/// `use_interval_fn` is a no-op, which is fine: the server-rendered frame is
/// correct for the instant it was rendered.
pub fn use_ticking_now() -> Signal<DateTime<Utc>> {
    let now = RwSignal::new(Utc::now());
    use_interval_fn(move || now.set(Utc::now()), 1000);
    now.into()
}

/// How long an arena has left, counted down live, or `None` once it is over.
///
/// `started_at` is when the arena actually opened and `duration_seconds` how
/// long it runs for, so the end is derivable exactly — there is no need for the
/// server to keep telling us.
pub fn use_arena_time_left(
    started_at: Signal<Option<DateTime<Utc>>>,
    duration_seconds: Signal<Option<i32>>,
) -> Signal<Option<chrono::TimeDelta>> {
    let now = use_ticking_now();
    Signal::derive(move || {
        let ends_at = started_at.get()? + chrono::Duration::seconds(duration_seconds.get()? as i64);
        let left = ends_at.signed_duration_since(now.get());
        (left.num_seconds() > 0).then_some(left)
    })
}

/// `m:ss` while under an hour, `h:mm` above it — an arena runs for hours but the
/// last minute is the interesting part.
pub fn format_time_left(left: chrono::TimeDelta) -> String {
    let total = left.num_seconds().max(0);
    if total >= 3600 {
        format!("{}h {:02}m", total / 3600, (total % 3600) / 60)
    } else {
        format!("{}:{:02}", total / 60, total % 60)
    }
}
