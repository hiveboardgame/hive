use crate::{
    common::{Direction, Hex, HexType, PieceType},
    components::atoms::{active::Active, last_move::LastMove, piece::Piece, target::Target},
    pages::play::TargetStack,
    providers::game_state::GameStateSignal,
};
use leptos::*;

#[component]
pub fn Hex(hex: Hex) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let target_stack = expect_context::<TargetStack>();
    let level_multiplier = move || match target_stack.0() {
        Some(pos) => {
            if hex.position == pos {
                13
            } else {
                1
            }
        }
        None => 1,
    };
    let expanded_sublevel =
        Signal::derive(move || hex.level.saturating_sub(1) * level_multiplier() + 1);
    let expanded_level = Signal::derive(move || hex.level * level_multiplier());

    match hex.kind {
        HexType::Active(_) => {
            if game_state
                .signal
                .get_untracked()
                .move_info
                .target_position
                .is_none()
                || hex.level == 0
            {
                view! { <Active position=hex.position level=expanded_level /> }
            } else {
                view! { <Active position=hex.position level=expanded_sublevel /> }
            }
        }
        HexType::Target => {
            if hex.level == 0 {
                view! { <Target position=hex.position level=hex.level /> }
            } else {
                view! { <Target position=hex.position level=expanded_sublevel /> }
            }
        }
        HexType::Tile(piece, piece_type) => match piece_type {
            PieceType::Board | PieceType::Covered | PieceType::History => {
                view! { <Piece piece=piece position=hex.position level=expanded_level piece_type=piece_type /> }
            }
            PieceType::Move => {
                view! { <Piece piece=piece position=hex.position level=expanded_sublevel piece_type=piece_type /> }
            }
            PieceType::Spawn => {
                view! { <Piece piece=piece position=hex.position level=hex.level piece_type=piece_type /> }
            }
            _ => {
                view! { <Piece piece=piece position=hex.position level=hex.level piece_type=piece_type /> }
            }
        },
        HexType::LastMove(Direction::To) => {
            view! { <LastMove position=hex.position level=expanded_level direction=Direction::To /> }
        }
        HexType::LastMove(Direction::From) => {
            if hex.level == 0 {
                view! { <LastMove position=hex.position level=hex.level direction=Direction::From /> }
            } else {
                view! { <LastMove position=hex.position level=expanded_sublevel direction=Direction::From /> }
            }
        }
    }
}
