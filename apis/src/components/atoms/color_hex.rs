use crate::i18n::*;
use hive_lib::Color;
use leptos::prelude::*;

/// Small flat hexagon tile (white/black) used to mark a player's color.
#[component]
pub fn ColorHex(
    #[prop(into)] color: Signal<Color>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
    let src = move || match color.get() {
        Color::White => "/assets/tiles/flat/white.svg",
        Color::Black => "/assets/tiles/flat/black.svg",
    };
    let alt = move || match color.get() {
        Color::White => t_string!(i18n, common.white).to_string(),
        Color::Black => t_string!(i18n, common.black).to_string(),
    };
    let style = move || {
        if color.get() == Color::White {
            "filter: drop-shadow(0 0 1px #3a3a3a) drop-shadow(0 0 1px #3a3a3a);"
        } else {
            "filter: drop-shadow(0 0 1px #f0ead6) drop-shadow(0 0 1px #f0ead6);"
        }
    };
    view! { <img src=src alt=alt class=format!("size-3 shrink-0 {extend_tw_classes}") style=style /> }
}
