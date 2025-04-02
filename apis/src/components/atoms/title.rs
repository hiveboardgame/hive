use crate::providers::{games::GamesSignal, refocus::RefocusSignal};
use leptos::prelude::*;
use leptos_meta::Title as T;

#[component]
pub fn Title() -> impl IntoView {
    let games = expect_context::<GamesSignal>();
    let focused = expect_context::<RefocusSignal>();
    let title_text = move || {
        let len = games.own.get().next_untimed.len()
            + games.own.get().next_realtime.len()
            + games.own.get().next_correspondence.len();
        if !focused.signal.get().focused && len > 0 {
            format!("({}) HiveGame.com", len)
        } else {
            String::from("HiveGame.com")
        }
    };

    view! { <T text=title_text /> }
}
