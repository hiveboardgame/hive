use crate::common::game_state;
use crate::common::hex::Direction;
use crate::common::hex_stack::HexStack;
use crate::common::{game_state::GameState, piece_type::PieceType};
use crate::molecules::piece_stack::PieceStack;

use hive_lib::piece::Piece;
use hive_lib::position::Position;
use leptos::*;

use super::hex_stack::HexStack as HexStackView;

#[component]
pub fn BoardPieces(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
    let state = move || game_state.get().state.get();
    let targets = move || game_state.get().target_positions.get();
    let last_move = move || game_state.get().state.get().board.last_move;

    // TODO get the BOARD_SIZE from board

    let board = move || {
        let mut board = Vec::new();
        let targets = targets();
        let last_move = last_move();
        log!("Targets: {:?}", targets);
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                let bug_stack = state().board.board.get(position).clone();
                if bug_stack.is_empty() {
                    if let (Some(from), _) = last_move {
                        if from == position {
                            let mut hs = HexStack::new_from_last_move(position);
                            if targets.contains(&position) {
                                hs.add_target();
                            }
                            board.push(hs);
                        } else {
                            if targets.contains(&position) {
                                board.push(HexStack::new_from_target(position));
                            }
                        }
                    } else {
                        if targets.contains(&position) {
                            board.push(HexStack::new_from_target(position));
                        }
                    }
                } else {
                    let mut hs = HexStack::new_from_bugstack(&bug_stack, position);
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
                    board.push(hs);
                }
            }
        }
        log!("Board: {:?}", board);
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
