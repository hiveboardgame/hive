use crate::{
    common::with_class,
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{build_static_render_model, HivegroundInteraction, HivegroundPaint},
    providers::Config,
};
use hive_lib::{GameType, State};
use leptos::prelude::*;

const PREVIEW_VIEWBOX: (f32, f32, f32, f32) = (1120.0, 626.0, 280.0, 140.0);

const PREVIEW_MOVES: &str =
    r"wB1; bQ wB1-; wQ \wB1; bB1 bQ-; wB2 \wQ; bB1 bQ; wB2 wQ; bG1 bQ-; wA1 -wQ";

#[component]
pub fn PreviewTiles() -> impl IntoView {
    let game_type = GameType::MLP.to_string();
    let state = State::new_from_str(PREVIEW_MOVES, &game_type).expect("PREVIEW_MOVES is legal");
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config().tile);
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let interaction = HivegroundInteraction::static_view();

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
        let base_classes =
            "flex relative flex-col items-center w-full max-w-72 h-auto aspect-[2/1] rounded-lg border border-black/10 shadow-sm place-self-center sm:max-w-80 dark:border-white/10";
        if is_using_custom() {
            base_classes.to_string()
        } else {
            with_class("ui-card-row", base_classes)
        }
    });

    let thumbnail_pieces = Memo::new(move |_| build_static_render_model(&state.board));
    let show_count = expect_context::<Config>().show_any_numbers();

    view! {
        <div class=container_classes style=background_style>
            <svg
                aria-hidden="true"
                viewBox=format!(
                    "{} {} {} {}",
                    PREVIEW_VIEWBOX.0,
                    PREVIEW_VIEWBOX.1,
                    PREVIEW_VIEWBOX.2,
                    PREVIEW_VIEWBOX.3,
                )
                class="size-full touch-none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <g>
                    <HivegroundStacks model=thumbnail_pieces paint interaction show_count />
                </g>
            </svg>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{hex_width, SvgPos, HEX_SIZE},
        components::molecules::hiveground_stack::{
            BADGE_HALF_EXTENT,
            BADGE_OFFSET_X,
            BADGE_OFFSET_Y,
        },
    };

    #[test]
    fn preview_position_is_legal_and_holds_two_stacks() {
        let state = State::new_from_str(PREVIEW_MOVES, &GameType::MLP.to_string())
            .expect("preview moves are legal");
        let tall_stacks = build_static_render_model(&state.board)
            .stacks
            .iter()
            .filter(|stack| stack.layers.len() > 1)
            .count();
        assert_eq!(tall_stacks, 2);
    }

    #[test]
    fn preview_window_fits_every_tile_and_badge() {
        let state = State::new_from_str(PREVIEW_MOVES, &GameType::MLP.to_string())
            .expect("preview moves are legal");
        let (left, top, width, height) = PREVIEW_VIEWBOX;
        for stack in &build_static_render_model(&state.board).stacks {
            let level = stack
                .layers
                .iter()
                .map(|layer| layer.level)
                .max()
                .unwrap_or(0);
            let (x, y) = SvgPos::center_for_level(stack.position, level, false);
            let tile_half_width = hex_width() / 2.0;
            let left_edge = x - tile_half_width;
            let right_edge = x + tile_half_width.max(BADGE_OFFSET_X + BADGE_HALF_EXTENT);
            let top_edge = y - HEX_SIZE.max(-BADGE_OFFSET_Y + BADGE_HALF_EXTENT);
            let bottom_edge = y + HEX_SIZE;
            assert!(
                left_edge >= left && right_edge <= left + width,
                "{x} outside"
            );
            assert!(
                top_edge >= top && bottom_edge <= top + height,
                "{y} outside"
            );
        }
    }
}
