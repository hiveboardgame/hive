use crate::{
    components::atoms::piece::PieceGlyph,
    hiveground::{HivegroundInteraction, HivegroundPaint, PieceShadow},
    providers::config::TileOptions,
};
use hive_lib::Position;
use leptos::prelude::*;

const GHOST_WIDTH: f32 = 60.0;
const GHOST_HEIGHT: f32 = 64.0;

#[component]
pub fn DragGhost(
    interaction: HivegroundInteraction,
    tile_opts: Signal<TileOptions>,
    board_scale: Signal<f32>,
) -> impl IntoView {
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let dragged_piece = Memo::new(move |_| interaction.dragging().map(|drag| drag.dragged.piece));
    let style = move || {
        let Some(drag) = interaction.dragging() else {
            return String::new();
        };
        let scale = board_scale.get();
        let (width, height) = (GHOST_WIDTH * scale, GHOST_HEIGHT * scale);
        let (x, y) = drag.pointer;
        format!(
            "left: {}px; top: {}px; width: {width}px; height: {height}px",
            x as f32 - width / 2.0,
            y as f32 - height / 2.0,
        )
    };

    move || {
        dragged_piece.get().map(|piece| {
            let piece_paint =
                Memo::new(move |_| paint.with(|paint| paint.piece(piece, PieceShadow::Design)));
            view! {
                <svg
                    class="fixed z-50 pointer-events-none"
                    style=style
                    viewBox=format!(
                        "{} {} {GHOST_WIDTH} {GHOST_HEIGHT}",
                        -GHOST_WIDTH / 2.0,
                        -GHOST_HEIGHT / 2.0,
                    )
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <PieceGlyph
                        position=Position::new(0, 0)
                        level=Signal::stored(0)
                        paint=piece_paint
                    />
                </svg>
            }
        })
    }
}
