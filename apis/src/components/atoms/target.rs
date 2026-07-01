use crate::{
    common::OverlayPaint,
    components::atoms::overlay::OverlayGlyph,
    hiveground::HivegroundInteraction,
};
use hudsoni::Position;
use leptos::prelude::*;

#[component]
pub fn Target(
    position: Position,
    paint: Memo<OverlayPaint>,
    level: Signal<usize>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    view! {
        <g on:click=move |evt| interaction.click_target(evt, position)>
            <OverlayGlyph position level paint />
        </g>
    }
}
