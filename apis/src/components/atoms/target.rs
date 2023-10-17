use crate::common::svg_pos::SvgPos;
use crate::providers::game_state::GameStateSignal;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Target(
    position: Position,
    level: usize,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let center = SvgPos::center_for_level(position, level);
    let transform = format!("translate({},{})", center.0, center.1);
    let mut game_state_signal = expect_context::<GameStateSignal>();

    // Select the target position
    let onclick = move |_| {
        game_state_signal.set_target(position);
    };

    view! {
        <g on:click=onclick class=format!("{extend_tw_classes}")>
            <g id="Target" transform=format!("{}", transform)>
                <use_
                    href="#target"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
