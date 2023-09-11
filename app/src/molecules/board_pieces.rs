use crate::common::game_state::GameStateSignal;
use crate::common::hex::Direction;
use crate::common::hex_stack::HexStack;

use hive_lib::{position::Position, color::Color};
use leptos::*;

use super::hex_stack::HexStack as HexStackView;

#[component]
pub fn BoardPieces(cx: Scope) -> impl IntoView {
    let game_state_signal =
        use_context::<RwSignal<GameStateSignal>>(cx).expect("there to be a `GameState` signal provided");

    // TODO get the BOARD_SIZE from board

    let board = move || {
        let mut board = Vec::new();
        let game_state = game_state_signal.get().signal.get();
        let targets = game_state.target_positions;
        let last_move = game_state.state.board.last_move;
        let active_piece = (game_state.active, game_state.position);
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                // start this empty and only add
                let bug_stack = game_state.state.board.board.get(position).clone();
                let mut hs = HexStack::new(&bug_stack, position);
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
                if targets.contains(&position) {
                    hs.add_target();
                }
                if let (Some(piece), Some(target_position)) = active_piece {
                    if position == target_position {
                        hs.add_active(piece);
                    }
                }
                if hs.len() > 0 {
                    board.push(hs);
                }
            }
        }
        board
    };

    move || {
        board()
            .into_iter()
            .map(|hs| {
                view! { cx, <HexStackView hex_stack=hs/> }
            })
            .collect_view(cx)
    }
}
