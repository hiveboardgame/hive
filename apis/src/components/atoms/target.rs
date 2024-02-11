use crate::common::move_confirm::MoveConfirm;
use crate::common::svg_pos::SvgPos;
use crate::providers::confirm_mode::ConfirmMode;
use crate::providers::game_state::GameStateSignal;
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn Target(
    position: Position,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let center = move || SvgPos::center_for_level(position, level());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let mut game_state_signal = expect_context::<GameStateSignal>();
    let confirm_mode = expect_context::<ConfirmMode>();

    // Select the target position and make a move if it's the correct mode
    let onclick = move |_| {
        if game_state_signal.is_move_allowed() {
            game_state_signal.set_target(position);
            if matches!((confirm_mode.preferred_confirm)(), MoveConfirm::Single) {
                game_state_signal.move_active();
            }
        }
    };

    view! {
        <g on:click=onclick class=extend_tw_classes>
            <g id="Target" transform=transform>
                <use_
                    href="#target"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
