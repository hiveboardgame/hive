use crate::common::svg_pos::SvgPos;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn LastMove(
    position: Position,
    level: usize,
    #[prop(default = "")] extend_tw_classes: &'static str,
) -> impl IntoView {
    let center = SvgPos::center_for_level(position, level);
    let transform = format!("translate({},{})", center.0, center.1);
    let onclick = move |_| {};

    view! {
        <g on:click=onclick class=format!("{extend_tw_classes}")>
            <g id="Lastmove" transform=format!("{}", transform)>
                <use_
                    href="#lastmove"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
