use crate::{
    common::OverlayPaint,
    components::atoms::overlay::OverlayGlyph,
    hiveground::HivegroundInteraction,
};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn Target(
    position: Position,
    paint: Memo<OverlayPaint>,
    level: Signal<usize>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let aria_label = format!("Move to board position {}, {}", position.q, position.r);
    view! {
        <g
            role="button"
            aria-label=aria_label
            on:click=move |evt| interaction.click_target(evt, position)
        >
            <OverlayGlyph position level paint />
        </g>
    }
}
