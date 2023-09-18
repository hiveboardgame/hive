use crate::common::{game_state::GameStateSignal, svg_pos::SvgPos};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Target(cx: Scope, position: Position, level: usize) -> impl IntoView {
    let center = SvgPos::center_for_level(position, level);
    let transform = format!("translate({},{})", center.0, center.1);
    let game_state_signal =
        use_context::<RwSignal<GameStateSignal>>(cx).expect("there to be a `GameState` signal provided");

    // Select the target position
    let onclick = move |_| {
        let mut game_state = game_state_signal.get();
        game_state.set_target(position);
    };

    view! { cx,
        <g on:click=onclick class="target">
            <g id="Target" transform=format!("{}", transform)>
                <use_
                    href="#target"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
