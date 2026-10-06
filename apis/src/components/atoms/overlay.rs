use crate::common::{OverlayPaint, SvgPos};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn OverlayGlyph(
    position: Position,
    level: Signal<usize>,
    paint: Memo<OverlayPaint>,
) -> impl IntoView {
    let center =
        move || paint.with(|paint| SvgPos::center_for_level(position, level(), paint.straight));
    let transform = move || format!("translate({},{})", center().0, center().1);
    let href = move || paint.with(|paint| paint.href);
    let color = move || paint.with(|paint| paint.color.map(|color| format!("color: {color}")));

    view! {
        <g transform=transform>
            <use_
                href=href
                style=color
                transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
            ></use_>
        </g>
    }
}
