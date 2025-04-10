use crate::{
    common::{Direction, Hex, HexType, PieceType},
    components::atoms::{active::Active, last_move::LastMove, piece::Piece, target::Target},
    providers::{config::TileOptions, game_state::GameStateSignal},
};
use hive_lib::Position;
use leptos::{either::EitherOf4, prelude::*};

#[component]
pub fn Hex(
    hex: Hex,
    tile_opts: TileOptions,
    target_stack: RwSignal<Option<Position>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let straight = tile_opts.clone().is_three_d();
    let level_multiplier = move || match target_stack() {
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
        HexType::Active(active_state) => {
            let level = if game_state
                .signal
                .get_untracked()
                .move_info
                .target_position
                .is_none()
                || hex.level == 0
            {
                expanded_level.get_untracked()
            } else {
                expanded_sublevel.get_untracked()
            };
            EitherOf4::A(view! { <Active position=hex.position level active_state straight /> })
        }
        HexType::Target => {
            let level = if hex.level == 0 {
                hex.level.into()
            } else {
                expanded_sublevel
            };
            EitherOf4::B(view! { <Target position=hex.position level straight /> })
        }
        HexType::Tile(piece, piece_type) => {
            let level = match piece_type {
                PieceType::Board | PieceType::Covered | PieceType::History => expanded_level,
                PieceType::Move => expanded_sublevel,
                _ => hex.level.into(),
            };
            EitherOf4::C(
                view! { <Piece piece=piece position=hex.position level=level tile_opts piece_type=piece_type /> },
            )
        }
        HexType::LastMove(direction) => {
            let level = match direction {
                Direction::To => expanded_level,
                Direction::From => {
                    if hex.level == 0 {
                        hex.level.into()
                    } else {
                        expanded_sublevel
                    }
                }
            };
            EitherOf4::D(view! { <LastMove position=hex.position level direction straight /> })
        }
    }
}
