use crate::common::MoveConfirm;
use crate::common::SvgPos;
use crate::components::organisms::analysis::AnalysisSignal;
use crate::pages::play::CurrentConfirm;
use crate::providers::game_state::GameStateSignal;
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
    let mut game_state = expect_context::<GameStateSignal>();
    let analysis = use_context::<AnalysisSignal>()
        .unwrap_or(AnalysisSignal(RwSignal::new(None)))
        .0;
    // Select the target position and make a move if it's the correct mode
    let onclick = move |_| {
        let in_analysis = analysis.get().is_some();
        if in_analysis || game_state.is_move_allowed() {
            game_state.set_target(position);
            if current_confirm() == MoveConfirm::Single || in_analysis {
                game_state.move_active();
            }
            analysis.update(|analysis| {
                if let Some(analysis) = analysis {
                    let state = game_state.signal.get_untracked().state;
                    let moves = state.history.moves;
                    let hashes = state.hashes;
                    let last_index = moves.len() - 1;
                    if moves[last_index].0 == "pass" {
                        //if move is pass, add prev move
                        analysis.add_node(moves[last_index - 1].clone(), hashes[last_index - 1]);
                    }
                    analysis.add_node(moves[last_index].clone(), hashes[last_index]);
                }
            });
        }
    };

    view! {
        <g on:click=onclick class=extend_tw_classes>
            <g id="Target" transform=transform>
                <use_
                    href="/assets/tiles/common/all.svg#target"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
