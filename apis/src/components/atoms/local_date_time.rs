use crate::common::format_tournament_datetime;
use chrono::{DateTime, Utc};
use leptos::prelude::*;

#[component]
pub fn LocalDateTime(
    date_time: DateTime<Utc>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    // The server's timezone is not the viewer's: the text is filled after hydration.
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));
    view! {
        <time class=class datetime=date_time.to_rfc3339()>
            {move || {
                if hydrated.get() { format_tournament_datetime(date_time) } else { String::new() }
            }}
        </time>
    }
}
