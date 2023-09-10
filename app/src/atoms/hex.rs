use crate::common::{
    hex::{Hex, HexType},
};

use leptos::*;

use crate::atoms::{last_move::LastMove, piece::Piece, target::Target};

#[component]
pub fn Hex(cx: Scope, hex: Hex) -> impl IntoView {
    match hex.kind {
        HexType::Tile(piece, piece_type) => {
            view! { cx,
                    <Piece piece=piece position=hex.position level=hex.level piece_type=piece_type/>
            }
        }
        HexType::LastMove => view! { cx,
            <LastMove position=hex.position level=hex.level/>
        },
        HexType::Target => view! { cx,
            <Target position=hex.position level=hex.level/>
        },
    }
}
