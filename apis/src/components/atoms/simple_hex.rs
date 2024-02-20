use crate::{
    common::hex::{Hex, HexType},
    components::atoms::{active::Active, last_move::LastMove, piece::Piece, target::Target},
};

use leptos::*;

#[component]
pub fn SimpleHex(hex: Hex) -> impl IntoView {
    match hex.kind {
        HexType::Active(_) => view! { <Active position=hex.position level=hex.level/> },
        HexType::Tile(piece, piece_type) => {
            view! { <Piece piece=piece position=hex.position level=hex.level piece_type=piece_type/> }
        }
        HexType::LastMove(dir) => {
            view! { <LastMove position=hex.position level=hex.level direction=dir/> }
        }
        HexType::Target => view! { <Target position=hex.position level=hex.level/> },
    }
}
