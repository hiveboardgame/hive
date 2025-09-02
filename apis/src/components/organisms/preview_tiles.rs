use crate::{
    common::HexStack, components::molecules::simple_hex_stack::SimpleHexStack, providers::Config,
};
use hive_lib::{GameType, Position, State};
use leptos::prelude::*;

#[component]
pub fn PreviewTiles() -> impl IntoView {
    let moves = "wA1; bG1 wA1-; wA2 /wA1; bG2 bG1\\; wA3 -wA1; bG3 bG1-";
    let state = State::new_from_str(moves, &GameType::MLP.to_string()).unwrap();
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config().tile);

    // Background styling logic
    let background_style = Signal::derive(move || {
        let is_dark_mode = config().prefers_dark;
        format!(
            "background-color: {}",
            config().tile.get_effective_background_color(is_dark_mode)
        )
    });

    let is_using_custom =
        Signal::derive(move || config.with(|c| !c.tile.is_using_custom_background(c.prefers_dark)));

    let container_classes = Signal::derive(move || {
        let base_classes = "flex relative flex-col items-center mx-1 my-2 w-72 h-36 rounded sm:h-40 sm:w-80 place-self-center";
        if is_using_custom() {
            base_classes.to_string()
        } else {
            format!("{base_classes} dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light")
        }
    });

    let thumbnail_pieces = move || {
        let mut pieces = Vec::new();
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                let bug_stack = state.board.board.get(position).clone();
                if !bug_stack.is_empty() {
                    pieces.push(HexStack::new_history(&bug_stack, position));
                }
            }
        }
        pieces
    };

    let pieces = move || {
        thumbnail_pieces()
            .into_iter()
            .map(|hs| {
                view! { <SimpleHexStack hex_stack=hs tile_opts=tile_opts() /> }
            })
            .collect_view()
    };

    view! {
        <div class=container_classes style=background_style>
            <svg
                viewBox="1159 695 225 100"
                class="size-full touch-none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <g>{pieces}</g>
            </svg>
        </div>
    }
}
