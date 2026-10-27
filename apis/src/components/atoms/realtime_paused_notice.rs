use crate::{i18n::*, providers::RealtimeGames};
use leptos::prelude::*;

#[component]
pub fn RealtimePausedNotice(#[prop(into, optional)] applies: MaybeProp<bool>) -> impl IntoView {
    let i18n = use_i18n();
    let realtime_games = expect_context::<RealtimeGames>();
    view! {
        <Show when=move || realtime_games.paused.get() && applies.get().unwrap_or(true)>
            <p
                role="status"
                class="p-3 text-sm bg-orange-50 rounded border border-orange-400 dark:text-orange-100 text-orange-950 dark:bg-orange-950"
            >
                {t!(i18n, messages.errors.realtime_games_paused)}
            </p>
        </Show>
    }
}
