use crate::i18n::{t_string, I18nKeys, Locale};
use chrono::{DateTime, TimeDelta, Utc};
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use leptos_use::use_interval_fn;

#[derive(Clone, Copy)]
pub struct TournamentTickingClock(pub Signal<DateTime<Utc>>);

pub fn use_ticking_now() -> Signal<DateTime<Utc>> {
    if let Some(clock) = use_context::<TournamentTickingClock>() {
        return clock.0;
    }
    let now = RwSignal::new(Utc::now());
    use_interval_fn(move || now.set(Utc::now()), 1000);
    now.into()
}

pub fn format_time_left(i18n: I18nContext<Locale, I18nKeys>, left: TimeDelta) -> String {
    let total = left.num_seconds().max(0);
    if total >= 3600 {
        t_string!(
            i18n,
            game.clock.arena_hours_minutes,
            hours = total / 3600,
            minutes = format!("{:02}", (total % 3600) / 60)
        )
        .to_string()
    } else {
        format!("{}:{:02}", total / 60, total % 60)
    }
}
