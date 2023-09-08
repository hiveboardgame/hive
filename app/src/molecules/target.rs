use crate::common::{game_state::GameState, svg_pos::SvgPos};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Target(cx: Scope, position: Position, level: usize) -> impl IntoView {
    let center = SvgPos::center_for_level(position, level);
    let transform = format!("translate({},{})", center.0, center.1);
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");

    // Select the target position
    let onclick = move |_| {
        game_state.get().position.set(Some(position));
        game_state.get().target_positions.set(Vec::new());
    };

    view! { cx,
        <g on:click = onclick class="target">
           <g id="Target" transform=format!("{}", transform)>
                <use_ href="#target" transform="scale(0.56, 0.56) translate(-50, -45)"/>
            </g>
        </g>
    }
}

#[component]
pub fn Targets(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
    let targets = move || game_state.get().target_positions.get();
    view! {cx,
        <For
        each=targets
        key=|target| (target.q, target.r)
        view=move |cx, target: Position| {
            view! {cx, <Target position=target level=0/>}
        }
      />
    }
}
