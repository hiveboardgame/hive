use crate::{
    common::SvgPos,
    components::molecules::annotations_layer::{rounded_hex_path, HIGHLIGHT_SIZE},
    hiveground::grid_positions,
    providers::{annotations::AnnotationColor, Config},
};
use hive_lib::Board;
use leptos::prelude::*;

#[component]
pub fn BoardGrid(#[prop(into)] board: Signal<Board>) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let cells = move || {
        let (straight, prefers_dark) = config.with(|c| (c.tile.is_three_d(), c.prefers_dark));
        let stroke = AnnotationColor::White.stroke(prefers_dark);
        board
            .with(grid_positions)
            .into_iter()
            .map(|position| {
                let (cx, cy) = SvgPos::center_for_level(position, 0, straight);
                view! {
                    <path
                        d=rounded_hex_path(cx, cy, HIGHLIGHT_SIZE)
                        fill="none"
                        stroke=stroke
                        stroke-width="1.5"
                        stroke-opacity="0.35"
                        stroke-linejoin="round"
                    />
                }
            })
            .collect_view()
    };

    view! { <g pointer-events="none">{cells}</g> }
}
