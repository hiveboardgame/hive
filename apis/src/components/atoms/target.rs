use crate::{
    common::OverlayPaint,
    components::atoms::overlay::OverlayGlyph,
    hiveground::HivegroundInteraction,
    i18n::*,
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
    let i18n = use_i18n();
    let aria_label = move || {
        t_string!(
            i18n,
            game.board.move_to_position,
            q = position.q,
            r = position.r
        )
    };
    view! {
        <g
            role="button"
            aria-label=aria_label
            data-hg-target-q=position.q
            data-hg-target-r=position.r
            on:click=move |evt| interaction.click_target(evt, position)
        >
            <OverlayGlyph position level paint />
        </g>
    }
}
