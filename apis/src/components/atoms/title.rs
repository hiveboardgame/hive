use crate::providers::{games::GamesSignal, refocus::RefocusSignal};
use leptos::prelude::*;
use leptos_meta::Title as T;

#[component]
pub fn Title() -> impl IntoView {
    let games = expect_context::<GamesSignal>();
    let focused = expect_context::<RefocusSignal>();
    let title_text = move || {
        games.own.with(|own| {
            let len =
                own.next_untimed.len() + own.next_realtime.len() + own.next_correspondence.len();
            focused.signal.with(|f| {
                if !f.focused && len > 0 {
                    format!("({len}) HiveGame.com")
                } else {
                    String::from("HiveGame.com")
                }
            })
        })
    };

    view! { <T text=title_text /> }
}
