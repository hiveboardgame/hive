use crate::{
    common::{Hex, HexType},
    components::atoms::simple_piece::SimplePiece,
};

use leptos::*;

#[component]
pub fn SimpleHex(hex: Hex) -> impl IntoView {
    if let HexType::Tile(piece, _) = hex.kind {
        view! { <SimplePiece piece=piece position=hex.position level=hex.level/> }
    } else {
        view! {}.into_view()
    }
}
