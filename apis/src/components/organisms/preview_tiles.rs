use crate::{
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{build_static_render_model, HivegroundInteraction, HivegroundPaint},
    providers::Config,
};
use hive_lib::{GameType, State};
use leptos::prelude::*;

#[component]
pub fn PreviewTiles() -> impl IntoView {
    let moves = "wA1; bG1 wA1-; wA2 /wA1; bG2 bG1\\; wA3 -wA1; bG3 bG1-";
    let state = State::new_from_str(moves, &GameType::MLP.to_string()).unwrap();
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config().tile);
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let interaction = HivegroundInteraction::static_view();

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

    let thumbnail_pieces = Memo::new(move |_| build_static_render_model(&state.board));

    view! {
        <div class=container_classes style=background_style>
            <svg
                viewBox="1159 695 225 100"
                class="size-full touch-none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <g>
                    <HivegroundStacks model=thumbnail_pieces paint interaction />
                </g>
            </svg>
        </div>
    }
}
