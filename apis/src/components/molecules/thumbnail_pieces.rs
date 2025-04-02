use crate::common::SvgPos;
use crate::common::TileDesign;
use crate::providers::Config;
use crate::{common::HexStack, components::molecules::simple_hex_stack::SimpleHexStack};
use hive_lib::Board;
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn ThumbnailPieces(board: StoredValue<Board>) -> impl IntoView {
    let thumbnail_pieces = move || {
        let mut pieces = Vec::new();
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                let bug_stack = board.get_value().board.get(position).clone();
                if !bug_stack.is_empty() {
                    pieces.push(HexStack::new_history(&bug_stack, position));
                }
            }
        }
        pieces
    };

    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config().tile);
    let straight = move || config().tile.design == TileDesign::ThreeD;

    let (width, height) = (400.0_f32, 510.0_f32);
    // TODO: because Thumbnail pieces is used in two places, this leads to weirdness in the TV
    let transform = move || {
        let svg_pos = SvgPos::center_for_level(board.get_value().center_coordinates(), 0, straight());
        let x_transform = -(svg_pos.0 - (width / 2.0));
        let y_transform = -(svg_pos.1 - (height / 2.0));
        format!("translate({},{})", x_transform, y_transform)
    };

    view! {
        <svg
            viewBox=format!("0 0 {width} {height}")
            class="w-full h-full touch-none"
            xmlns="http://www.w3.org/2000/svg"
        >
            <g transform=transform>
                {move || {
                    thumbnail_pieces()
                        .into_iter()
                        .map(|hs| {
                            view! { <SimpleHexStack hex_stack=hs tile_opts=tile_opts() /> }
                        })
                        .collect_view()
                }}

            </g>
        </svg>
    }
}
