use crate::{
    common::{SvgPos, TileDesign},
    components::molecules::hiveground_stacks::HivegroundStacks,
    hiveground::{
        build_static_render_model,
        HivegroundInteraction,
        HivegroundPaint,
        HivegroundRenderModel,
    },
    providers::Config,
};
use hudsoni::Board;
use leptos::prelude::*;

const VIEWBOX_WIDTH: f32 = 400.0;
const VIEWBOX_HEIGHT: f32 = 510.0;
const TILE_WIDTH: f32 = 88.337 * 0.56;
const TILE_HEIGHT: f32 = 104.229 * 0.56;
const VIEWBOX_PADDING: f32 = 20.0;
const MAX_THUMBNAIL_SCALE: f32 = 1.0;

#[component]
pub fn ThumbnailPieces(board: StoredValue<Board>) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config.with(|c| c.tile.clone()));
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let interaction = HivegroundInteraction::static_view();
    let thumbnail_pieces = Memo::new(move |_| board.with_value(build_static_render_model));

    let straight = move || config.with(|c| c.tile.design == TileDesign::ThreeD);

    let transform = move || thumbnail_pieces.with(|model| thumbnail_transform(model, straight()));

    view! {
        <svg
            viewBox=format!("0 0 {VIEWBOX_WIDTH} {VIEWBOX_HEIGHT}")
            class="block overflow-hidden size-full touch-none"
            xmlns="http://www.w3.org/2000/svg"
        >
            <g transform=transform>
                <HivegroundStacks model=thumbnail_pieces paint interaction />

            </g>
        </svg>
    }
}

fn thumbnail_transform(model: &HivegroundRenderModel, straight: bool) -> String {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;

    for stack in &model.stacks {
        for layer in &stack.layers {
            let (x, y) = SvgPos::center_for_level(stack.position, layer.level, straight);
            bounds = Some(match bounds {
                Some((min_x, min_y, max_x, max_y)) => {
                    (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                }
                None => (x, y, x, y),
            });
        }
    }

    let Some((min_x, min_y, max_x, max_y)) = bounds else {
        return format!(
            "translate({},{})",
            VIEWBOX_WIDTH / 2.0,
            VIEWBOX_HEIGHT / 2.0
        );
    };

    let content_width = (max_x - min_x) + TILE_WIDTH;
    let content_height = (max_y - min_y) + TILE_HEIGHT;
    let scale = ((VIEWBOX_WIDTH - 2.0 * VIEWBOX_PADDING) / content_width)
        .min((VIEWBOX_HEIGHT - 2.0 * VIEWBOX_PADDING) / content_height)
        .min(MAX_THUMBNAIL_SCALE);
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    format!(
        "translate({:.2},{:.2}) scale({:.4}) translate({:.2},{:.2})",
        VIEWBOX_WIDTH / 2.0,
        VIEWBOX_HEIGHT / 2.0,
        scale,
        -center_x,
        -center_y,
    )
}
