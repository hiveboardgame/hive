use crate::{
    common::{hex::Direction, hex_stack::HexStack},
    components::molecules::hex_stack::HexStack as HexStackView,
    providers::game_state::GameStateSignal,
};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn BoardPieces() -> impl IntoView {
    let game_state_signal =
        use_context::<GameStateSignal>().expect("there to be a `GameState` signal provided");

    // TODO get the BOARD_SIZE from board

    let board = move || {
        let mut board = Vec::new();
        let game_state = game_state_signal.signal.get();
        let targets = game_state.target_positions;
        let last_move = game_state.state.board.last_move;
        let active_piece = (game_state.active, game_state.target_position);
        let from_to_position = (game_state.current_position, game_state.target_position);
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                // start this empty and only add
                let bug_stack = game_state.state.board.board.get(position).clone();
                let mut hs = HexStack::new(&bug_stack, position);
                if game_state.active.is_none() {
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
                if let (Some(piece), Some(target_position)) = active_piece {
                    if position == target_position {
                        hs.add_tile(piece);
                    }
                }
                if !hs.is_empty() {
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
                view! { <HexStackView hex_stack=hs/> }
            })
            .collect_view()
    }
}
