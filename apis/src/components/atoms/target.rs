use crate::common::MoveConfirm;
use crate::common::SvgPos;
use crate::common::TileDesign;
use crate::components::organisms::analysis::AnalysisSignal;
use crate::pages::game::CurrentConfirm;
use crate::providers::game_state::GameStateSignal;
use crate::providers::Config;
use hive_lib::Position;
use leptos::*;

#[component]
pub fn Target(
    position: Position,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let config = expect_context::<Config>();
    let straight = move || (config.tile_design.preferred_tile_design)() == TileDesign::ThreeD;
    let center = move || SvgPos::center_for_level(position, level(), straight());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let mut game_state = expect_context::<GameStateSignal>();
    let analysis = use_context::<AnalysisSignal>()
        .unwrap_or(AnalysisSignal(RwSignal::new(None)))
        .0;
    let current_confirm = expect_context::<CurrentConfirm>().0;
    // Select the target position and make a move if it's the correct mode
    let onclick = move |_| {
        let in_analysis = analysis.get().is_some();
        if in_analysis || game_state.is_move_allowed() {
            batch(move || {
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
                            analysis
                                .add_node(moves[last_index - 1].clone(), hashes[last_index - 1]);
                        }
                        analysis.add_node(moves[last_index].clone(), hashes[last_index]);
                    }
                });
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
