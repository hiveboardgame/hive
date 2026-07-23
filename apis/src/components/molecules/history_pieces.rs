use crate::{
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{build_static_render_model, HivegroundInteraction, HivegroundPaint},
    providers::config::TileOptions,
};
use hive_lib::Board;
use leptos::prelude::*;

#[component]
pub fn HistoryPieces(
    tile_opts: Signal<TileOptions>,
    interaction: HivegroundInteraction,
    history_board: Memo<Board>,
) -> impl IntoView {
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));

    let history_pieces = Memo::new(move |_| history_board.with(build_static_render_model));

    view! { <HivegroundStacks model=history_pieces paint interaction /> }
}
