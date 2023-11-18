use crate::common::svg_pos::SvgPos;
use crate::providers::game_state::GameStateSignal;
use hive_lib::position::Position;
use leptos::*;
use leptos::logging::log;

#[component]
pub fn Target(
    position: Position,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let center = move || SvgPos::center_for_level(position, level());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let mut game_state_signal = expect_context::<GameStateSignal>();

    // Select the target position
    let onclick = move |_| {
        log!("Target piece");
        game_state_signal.set_target(position);
    };

    view! {
        <g on:click=onclick class=format!("{extend_tw_classes}")>
            <g id="Target" transform=transform>
                <use_
                    href="#target"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}

