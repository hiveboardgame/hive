use crate::{
    common::{Hex, HexType},
    components::atoms::piece::Piece,
};
use leptos::{either::Either, prelude::*};

#[component]
pub fn SimpleHex(hex: Hex) -> impl IntoView {
    if let HexType::Tile(piece, _) = hex.kind {
        Either::Left(view! { <Piece piece=piece position=hex.position level=hex.level simple=true /> })
    } else {
        Either::Right(())
    }
}
