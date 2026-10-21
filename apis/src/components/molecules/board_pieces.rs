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
    let tutorial = use_context::<crate::providers::tutorial::TutorialContext>();
    let paint = Memo::new(move |_| {
        let movers = tutorial.and_then(|tutorial| {
            tutorial.session.with(|session| {
                session.as_ref().and_then(|session| session.lesson().blank_movers())
            })
        });
        let visible = tutorial.and_then(|tutorial| {
            tutorial.session.with(|session| {
                session.as_ref().and_then(|session| session.lesson().blank_visible())
            })
        });
        let revealed = tutorial.map_or_else(Vec::new, |tutorial| {
            tutorial.session.with(|session| {
                session.as_ref().map_or_else(Vec::new, |session| session.revealed_neighbours())
            })
        });
        tile_opts.with(HivegroundPaint::new)
            .with_blank_movers(movers)
            .with_visible_pieces(visible)
            .with_revealed_pieces(&revealed)
    });
    let model = Memo::new(move |_| {
        state.with(|state| {
            move_info.with(|move_info| build_board_render_model(&state.board, move_info))
        })
    });

    view! { <HivegroundStacks model paint interaction /> }
}
