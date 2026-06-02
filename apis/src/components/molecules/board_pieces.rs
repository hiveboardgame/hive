use crate::{
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{build_board_render_model, HivegroundInteraction, HivegroundPaint},
    providers::{config::TileOptions, game_state::GameStateSignal},
};
use leptos::prelude::*;

#[component]
pub fn BoardPieces(
    tile_opts: Signal<TileOptions>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let move_info = create_read_slice(game_state.signal, |gs| gs.move_info.clone());
    let state = create_read_slice(game_state.signal, |gs| gs.state.clone());
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let model = Memo::new(move |_| {
        move_info.with(|move_info| state.with(|state| build_board_render_model(state, move_info)))
    });

    view! { <HivegroundStacks model paint interaction /> }
}
