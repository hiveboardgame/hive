use crate::common::MoveConfirm;
use crate::common::SvgPos;
use crate::pages::{analysis::InAnalysis, play::CurrentConfirm};
use crate::providers::game_state::GameStateSignal;
use hive_lib::Position;
use leptos::*;

#[component]
pub fn Target(
    position: Position,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let center = move || SvgPos::center_for_level(position, level());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let mut game_state = expect_context::<GameStateSignal>();
    let in_analysis = use_context::<InAnalysis>().unwrap_or(InAnalysis(RwSignal::new(false)));
    let current_confirm = expect_context::<CurrentConfirm>().0;

    // Select the target position and make a move if it's the correct mode
    let onclick = move |_| {
        let in_analysis = in_analysis.0.get_untracked();
        if in_analysis || game_state.is_move_allowed() {
            batch(move || {
                game_state.set_target(position);
                if current_confirm() == MoveConfirm::Single || in_analysis {
                    game_state.move_active();
                }
            });
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
