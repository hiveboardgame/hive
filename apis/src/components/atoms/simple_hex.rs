use crate::{
    common::{Hex, HexType},
    components::atoms::piece::Piece,
    providers::config::TileOptions,
};
use leptos::{either::Either, prelude::*};

#[component]
pub fn SimpleHex(hex: Hex, tile_opts: TileOptions) -> impl IntoView {
    if let HexType::Tile(piece, _) = hex.kind {
        Either::Left(
            view! { <Piece piece=piece position=hex.position level=hex.level tile_opts simple=true /> },
        )
    } else {
        Either::Right(())
    }
}
