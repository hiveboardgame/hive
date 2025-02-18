use crate::{
    common::{Direction, HexStack, PieceType},
    components::molecules::hex_stack::HexStack as HexStackView,
    providers::game_state::GameStateSignal,
};
use leptos::*;

#[component]
pub fn BoardPieces() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    // TODO get the BOARD_SIZE from board
    let board = move || {
        let mut board = Vec::new();
        let game_state = (game_state.signal)();
        let targets = game_state.move_info.target_positions;
        let last_move = game_state.state.board.last_move;
        let active_piece = (
            game_state.move_info.active,
            game_state.move_info.target_position,
        );
        let from_to_position = (
            game_state.move_info.current_position,
            game_state.move_info.target_position,
        );

        for position in game_state.state.board.positions.iter().flatten() {
            let bug_stack = game_state.state.board.board.get(*position).clone();
            let mut hs = HexStack::new(&bug_stack, *position);
            if game_state.move_info.active.is_none() {
                if let (_, Some(to)) = last_move {
                    if to == *position {
                        hs.add_last_move(Direction::To);
                    }
                }
                if let (Some(from), _) = last_move {
                    if from == *position {
                        hs.add_last_move(Direction::From);
                    }
                }
            }
            if let (Some(from), to) = from_to_position {
                if *position == from {
                    hs.add_active(to.is_some());
                }
            }
            if targets.contains(position) {
                hs.add_target();
            }
            if let (Some(piece), Some(target_position)) = active_piece {
                if *position == target_position {
                    // Check here whether piece is still in reserve?
                    if game_state
                        .state
                        .current_reserve()
                        .contains_key(&piece.bug())
                    {
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
        board
    };

    move || {
        board()
            .into_iter()
            .map(|hs| {
                view! { <HexStackView hex_stack=hs /> }
            })
            .collect_view()
    }
}
