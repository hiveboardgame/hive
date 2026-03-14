use crate::{
    common::{MoveConfirm, SvgPos},
    pages::play::CurrentConfirm,
    providers::{analysis::AnalysisStore, game_state::GameStateStore, ApiRequestsProvider},
};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn Target(
    position: Position,
    straight: bool,
    #[prop(into)] level: Signal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let center = move || SvgPos::center_for_level(position, level(), straight);
    let transform = move || format!("translate({},{})", center().0, center().1);
    let game_state = expect_context::<GameStateStore>();
    let analysis = use_context::<AnalysisStore>();
    let api = expect_context::<ApiRequestsProvider>().0;
    // Select the target position and make a move if it's the correct mode
    let onclick = move |_| {
        if game_state.is_move_allowed(analysis.is_some()) {
            game_state.set_target(position);
            if current_confirm.get_untracked() == MoveConfirm::Single {
                game_state.move_active(analysis.clone(), api());
            }
        }
    };

    let href = || "/assets/tiles/common/all.svg#target";

    view! {
        <g on:click=onclick class=extend_tw_classes>
            <g id="Target" transform=transform>
                <use_ href=href transform="scale(0.56, 0.56) translate(-46.608, -52.083)"></use_>
            </g>
        </g>
    }
}
