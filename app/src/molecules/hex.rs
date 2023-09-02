// use crate::common::hex::{Hex, HexType};
// use hive_lib::{
//     board::Board, bug::Bug, color::Color, game_type::GameType, piece::Piece, position::Position,
// };
// use leptos::*;
// use crate::molecules::piece::Piece;
//
// #[component]
// pub fn Hex(cx: Scope, hex: Hex) -> impl IntoView {
//     match hex.kind {
//         HexType::Piece => view! { cx,
//             Piece<piece=hex.piece.unwrap() position=hex.position level=hex.level piece_type=hex.piece_type/>
//         },
//         HexType::LastMove => view! { cx,
//             <g class="destination">
//                 <use_ class="destination" href="#destination" transform="scale(0.56, 0.56) translate(-46.608, -52.083)" />
//             </g>
//         },
//         HexType::Destination => view! { cx,
//             <g class="destination">
//                 <use_ class="destination" href="#destination" transform="scale(0.56, 0.56) translate(-46.608, -52.083)" />
//             </g>
//         },
//     }
// }
