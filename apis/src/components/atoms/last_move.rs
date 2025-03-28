use crate::common::{Direction, SvgPos};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn LastMove(
    position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
    direction: Direction,
    straight: bool,
) -> impl IntoView {
    let center = move || SvgPos::center_for_level(position, level(), straight);
    let transform = move || format!("translate({},{})", center().0, center().1);
    let href = move || match direction {
        Direction::To => {
            if straight {
                "/assets/tiles/3d/last_move_to.svg#last_move_to"
            } else {
                "/assets/tiles/common/all.svg#last_move_to"
            }
        }
        Direction::From => "/assets/tiles/common/all.svg#last_move_from",
    };

    view! {
        <g class=format!("{extend_tw_classes}") transform=transform>
            <g transform="scale(0.56, 0.56) translate(-46.608, -52.083)">
                <use_ href=href></use_>
            </g>
        </g>
    }
}
