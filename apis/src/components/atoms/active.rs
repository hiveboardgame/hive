use crate::{
    common::OverlayPaint,
    components::atoms::overlay::OverlayGlyph,
    hiveground::{ActiveMarkerState, HivegroundInteraction},
};
use hive_lib::Position;
use leptos::{either::Either, prelude::*};

#[component]
pub fn Active(
    position: Position,
    level: Signal<usize>,
    active_state: ActiveMarkerState,
    paint: Memo<OverlayPaint>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    match active_state {
        ActiveMarkerState::None | ActiveMarkerState::Board => Either::Left(view! {
            <g on:click=move |evt| interaction.click_active(evt)>
                <OverlayGlyph position level paint />
            </g>
        }),
        ActiveMarkerState::Reserve => {
            Either::Right(view! { <OverlayGlyph position level paint /> })
        }
    }
}
