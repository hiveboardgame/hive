use crate::{
    common::{Direction, HexStack, PieceType},
    components::molecules::hex_stack::HexStack as HexStackView,
    providers::{config::TileOptions, game_state::GameStateSignal},
};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn BoardPieces(
    tile_opts: TileOptions,
    target_stack: RwSignal<Option<Position>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let move_info = create_read_slice(game_state.signal, |gs| gs.move_info.clone());
    let state = create_read_slice(game_state.signal, |gs| gs.state.clone());
    // TODO get the BOARD_SIZE from board
    let board = move || {
        let move_info = move_info();
        state.with(|state| {
            let mut board = Vec::new();
            let targets = move_info.target_positions;
            let last_move = state.board.last_move;
            let active_piece = (move_info.active, move_info.target_position);
            let from_to_position = (move_info.current_position, move_info.target_position);
            // TODO: Find a better solution instead of the nested loop here
            for r in 0..32 {
                for q in 0..32 {
                    let position = Position::new(q, r);
                    // start this empty and only add
                    let bug_stack = state.board.board.get(position);
                    let mut hs = HexStack::new(bug_stack, position);
                    if move_info.active.is_none() {
                        if let (_, Some(to)) = last_move {
                            if to == position {
                                hs.add_last_move(Direction::To);
                            }
                        }
                        if let (Some(from), _) = last_move {
                            if from == position {
                                hs.add_last_move(Direction::From);
                            }
                        }
                    }
                    if let (Some(from), to) = from_to_position {
                        if position == from {
                            hs.add_active(to.is_some());
                        }
                    }
                    if targets.contains(&position) {
                        hs.add_target();
                    }
                    if let (Some((piece, _)), Some(target_position)) = active_piece {
                        if position == target_position {
                            // Check here whether piece is still in reserve?
                            if state.current_reserve().contains_key(&piece.bug()) {
                                hs.add_tile(piece, PieceType::Spawn);
                            } else {
                                hs.add_tile(piece, PieceType::Move);
                            }
                        }
                    }
                    if !hs.is_empty() {
                        board.push(hs);
                    }
                }
            }
            board
        })
    };

    move || {
        board()
            .into_iter()
            .map(|hs| {
                view! { <HexStackView hex_stack=hs tile_opts=tile_opts.clone() target_stack /> }
            })
            .collect_view()
    }
}
