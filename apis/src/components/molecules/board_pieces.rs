use crate::{
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{build_board_render_model, HivegroundInteraction, HivegroundPaint},
    providers::{
        config::TileOptions,
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
    let model = Memo::new(move |_| {
        state.with(|state| {
            move_info.with(|move_info| build_board_render_model(&state.board, move_info))
        })
    });

    view! { <HivegroundStacks model paint interaction /> }
}
