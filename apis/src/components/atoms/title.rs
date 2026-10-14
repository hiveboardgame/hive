use crate::providers::{games::GamesSignal, refocus::RefocusSignal};
use leptos::prelude::*;
use leptos_meta::Title as T;
use leptos_router::hooks::use_location;

fn page_name(path: &str) -> Option<&'static str> {
    match path {
        "/puzzles/daily" => Some("Puzzle of the day"),
        "/puzzles/dailies" => Some("Past puzzles of the day"),
        "/puzzles/browse" => Some("Puzzle sets"),
        "/puzzles/history" => Some("Puzzle history"),
        path if path == "/puzzles" || path.starts_with("/puzzles/") => Some("Hive puzzle"),
        _ => None,
    }
}

#[component]
pub fn Title() -> impl IntoView {
    let games = expect_context::<GamesSignal>();
    let focused = expect_context::<RefocusSignal>();
    let pathname = use_location().pathname;
    let title_text = move || {
        let site = match pathname.with(|path| page_name(path)) {
            Some(page) => format!("{page} · HiveGame.com"),
            None => String::from("HiveGame.com"),
        };
        games.own.with(|own| {
            let len =
                own.next_untimed.len() + own.next_realtime.len() + own.next_correspondence.len();
            focused.signal.with(|f| {
                if !f.focused && len > 0 {
                    format!("({len}) {site}")
                } else {
                    site
                }
            })
        })
    };

    view! { <T text=title_text /> }
}
