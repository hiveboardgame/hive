use crate::{
    common::{OverlayPaint, PieceType},
    components::atoms::overlay::OverlayGlyph,
    hiveground::{ActiveMarkerState, HivegroundInteraction},
};
use hive_lib::{Piece, Position};
use leptos::{either::Either, prelude::*};
use web_sys::PointerEvent;

#[component]
pub fn Active(
    position: Position,
    level: Signal<usize>,
    active_state: ActiveMarkerState,
    source: Option<(Piece, PieceType)>,
    paint: Memo<OverlayPaint>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let press = move |evt: PointerEvent| {
        if let Some((piece, piece_type)) = source {
            interaction.press_piece(evt, piece, position, piece_type);
        }
    };
    match active_state {
        ActiveMarkerState::None | ActiveMarkerState::Board => Either::Left(view! {
            <g on:click=move |evt| interaction.click_active(evt) on:pointerdown=press>
                <OverlayGlyph position level paint />
            </g>
        }),
        ActiveMarkerState::Reserve => Either::Right(view! {
            <g on:pointerdown=press>
                <OverlayGlyph position level paint />
            </g>
        }),
    }
}
