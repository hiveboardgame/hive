use crate::{
    common::{SvgPos, TileDesign},
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{build_static_render_model, HivegroundInteraction, HivegroundPaint},
    providers::Config,
};
use hive_lib::Board;
use leptos::prelude::*;

#[component]
pub fn ThumbnailPieces(board: StoredValue<Board>) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config.with(|c| c.tile.clone()));
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let interaction = HivegroundInteraction::static_view();
    let thumbnail_pieces = Memo::new(move |_| board.with_value(build_static_render_model));

    let straight = move || config.with(|c| c.tile.design == TileDesign::ThreeD);

    let (width, height) = (400.0_f32, 510.0_f32);
    // TODO: because Thumbnail pieces is used in two places, this leads to weirdness in the TV
    let transform = move || {
        let svg_pos =
            SvgPos::center_for_level(board.read_value().center_coordinates(), 0, straight());
        let x_transform = -(svg_pos.0 - (width / 2.0));
        let y_transform = -(svg_pos.1 - (height / 2.0));
        format!("translate({x_transform},{y_transform})")
    };

    view! {
        <svg
            viewBox=format!("0 0 {width} {height}")
            class="size-full touch-none"
            xmlns="http://www.w3.org/2000/svg"
        >
            <g transform=transform>
                <HivegroundStacks model=thumbnail_pieces paint interaction />

            </g>
        </svg>
    }
}
