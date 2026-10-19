use crate::functions::game_evals::get_recent_evals;
use leptos::{leptos_dom::helpers::set_interval_with_handle, prelude::*};
use shared_types::RecentEvals;
use std::time::Duration;

const REFRESH_EVERY: Duration = Duration::from_secs(60);

pub type RecentEvalsFeed = Resource<Option<Result<RecentEvals, ServerFnError>>>;

/// One feed for every place the home page shows it, refreshed every minute; nothing is
/// fetched for viewers who cannot see evals.
pub fn use_recent_evals(visible: Signal<bool>) -> RecentEvalsFeed {
    let refresh = RwSignal::new(0u32);
    let feed = Resource::new(
        move || (visible.get(), refresh.get()),
        |(visible, _)| async move {
            if visible {
                Some(get_recent_evals().await)
            } else {
                None
            }
        },
    );
    Effect::new(move |_| {
        let tick = move || {
            if visible.get_untracked() {
                refresh.update(|n| *n += 1);
            }
        };
        if let Ok(handle) = set_interval_with_handle(tick, REFRESH_EVERY) {
            on_cleanup(move || handle.clear());
        }
    });
    feed
}
