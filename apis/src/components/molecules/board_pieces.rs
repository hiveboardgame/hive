use crate::{
    components::molecules::{
        annotations_layer::selected_suggestion,
        hiveground_stacks::HivegroundStacks,
    },
    hiveground::{build_board_render_model_with, HivegroundInteraction, HivegroundPaint},
    providers::{
        analysis::AnalysisContext,
        config::TileOptions,
        game_eval::GameEvalContext,
        game_state::{GameStateStore, GameStateStoreFields},
    },
};
use leptos::prelude::*;

#[component]
pub fn BoardPieces(
    tile_opts: Signal<TileOptions>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let move_info = game_state.move_info();
    let state = game_state.state();
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let analysis = use_context::<AnalysisContext>();
    let evals = use_context::<GameEvalContext>();
    // The pink last-move mark next to the eval's red move read as a third marking.
    let eval_shown = Memo::new(move |_| {
        !selected_suggestion(analysis, evals, game_state)
            .ghosts
            .is_empty()
    });
    let model = Memo::new(move |_| {
        let last_move = !eval_shown.get();
        state.with(|state| {
            move_info
                .with(|move_info| build_board_render_model_with(&state.board, move_info, last_move))
        })
    });

    view! { <HivegroundStacks model paint interaction /> }
}
