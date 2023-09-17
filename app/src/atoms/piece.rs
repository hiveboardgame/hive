use crate::common::{game_state::GameStateSignal, piece_type::PieceType, svg_pos::SvgPos};
use hive_lib::{color::Color, piece::Piece, position::Position};
use leptos::*;

#[component]
pub fn Piece(
    cx: Scope,
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
) -> impl IntoView {
    let center = SvgPos::center_for_level(position.get(), level.get());
    let transform = format!("translate({},{})", center.0, center.1);

    let mut filter = String::new();
    if piece.get().color() == Color::White {
        filter.push_str("drop-shadow-w");
    } else {
        filter.push_str("drop-shadow-b");
    }
    if piece_type == PieceType::Inactive {
        filter.push_str(" sepia");
    }
    let color = piece.get().color().to_string();
    let bug = piece.get().bug().to_string();
    // let order = piece.order().to_string();

    let game_state_signal = use_context::<RwSignal<GameStateSignal>>(cx)
        .expect("there to be a `GameState` signal provided");

    let onclick = move |_| {
        let mut game_state = game_state_signal.get();
        match piece_type {
            PieceType::Board => {
                log!("Board piece");
                game_state.show_moves(piece.get(), position.get());
            }
            PieceType::Reserve => {
                log!("Reserve piece");
                game_state.show_spawns(piece.get(), position.get());
            }
            PieceType::Spawn => {
                log!("Spawning piece {}", piece.get());
                game_state.spawn_active_piece();
            }
            _ => log!("Piece is {}", piece_type),
        }
    };

    view! { cx,
        <g on:click = onclick class={filter}>
           <g transform=format!("{}", transform)>
                <use_ href=format!("#{}", color) transform="scale(0.56, 0.56) translate(-45, -50)" />
                <use_ href=format!("#{}", bug) transform="scale(0.56, 0.56) translate(-50, -45)"/>
                // <use_ href=format!("#{}", order) transform="scale(0.56, 0.56) translate(-50, -45)"/>
            </g>
        </g>
    }
}
