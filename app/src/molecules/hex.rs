use crate::common::hex::{Hex, HexType};
use crate::molecules::piece::Piece;
use hive_lib::{
    board::Board, bug::Bug, color::Color, game_type::GameType, piece::Piece, position::Position,
};
use leptos::*;

#[component]
pub fn Hex(cx: Scope, hex: Hex) -> impl IntoView {
    match hex.kind {
        HexType::Tile(piece, piece_type) => view! { cx,
            <Piece piece=piece position=hex.position level=hex.level piece_type=piece_type/>
        },
        HexType::LastMove => view! { cx,
            <g class="destination">
                <use_ class="destination" href="#destination" transform="scale(0.56, 0.56) translate(-46.608, -52.083)" />
            </g>
        }.into_view(cx),
        HexType::Target => view! { cx,
            <g class="destination">
                <use_ class="destination" href="#destination" transform="scale(0.56, 0.56) translate(-46.608, -52.083)" />
            </g>
        }.into_view(cx),
    }
}
