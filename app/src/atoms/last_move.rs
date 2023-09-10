use crate::common::{game_state::GameState, svg_pos::SvgPos};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn LastMove(cx: Scope, position: Position, level: usize) -> impl IntoView {
    let center = SvgPos::center_for_level(position, level);
    let transform = format!("translate({},{})", center.0, center.1);

    view! { cx,
        <g class="lastmove">
            <g id="lastmove" transform=transform>
                <use_ href="#lastmove" transform="scale(0.56, 0.56) translate(-45, -50)"></use_>
            </g>
        </g>
    }
}
