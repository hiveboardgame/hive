use crate::common::{hex::Direction, svg_pos::SvgPos};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn LastMove(
    position: Position,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
    direction: Direction,
) -> impl IntoView {
    let center = move || SvgPos::center_for_level(position, level());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let href = match direction {
        Direction::To => "#lastmove_to",
        Direction::From => "#lastmove_from",
    };

    view! {
        <g class=format!("{extend_tw_classes}")>
            <g id="Lastmove" transform=transform>
                <use_ href=href transform="scale(0.56, 0.56) translate(-46.608, -52.083)"></use_>
            </g>
        </g>
    }
}
