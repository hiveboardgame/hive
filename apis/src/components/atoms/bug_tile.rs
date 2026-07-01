use crate::{
    common::{resolve_piece_paint, ShadowHref},
    components::atoms::piece::PieceGlyph,
    providers::Config,
};
use hudsoni::{Piece, Position};
use leptos::prelude::*;

/// A small standalone tile icon for a single piece, rendered with the user's current tile design.
/// Used inline (e.g. in front of a move's notation in the opening explorer). No shadow, no click.
#[component]
pub fn BugTile(piece: Piece) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let paint = Memo::new(move |_| {
        let tile_opts = config.with(|c| c.tile.clone());
        resolve_piece_paint(piece, &tile_opts, ShadowHref::None)
    });
    view! {
        <svg
            class="inline-block align-middle shrink-0 size-5"
            viewBox="-32 -34 64 68"
            xmlns="http://www.w3.org/2000/svg"
        >
            <PieceGlyph position=Position::new(0, 0) level=Signal::derive(|| 0) paint />
        </svg>
    }
}
