use crate::responses::game::GameResponse;
use crate::{common::hex_stack::HexStack, components::molecules::simple_hex_stack::SimpleHexStack};
use hive_lib::position::Position;
use leptos::*;

#[component]
pub fn ThumbnailPieces(game: GameResponse) -> impl IntoView {
    let state = game.create_state();
    let thumbnail_pieces = move || {
        let mut pieces = Vec::new();
        for r in 0..32 {
            for q in 0..32 {
                let position = Position::new(q, r);
                let bug_stack = state.board.board.get(position).clone();
                if !bug_stack.is_empty() {
                    pieces.push(HexStack::new_history(&bug_stack, position));
                }
            }
        }
        pieces
    };

    move || {
        thumbnail_pieces()
            .into_iter()
            .map(|hs| {
                view! { <SimpleHexStack hex_stack=hs/> }
            })
            .collect_view()
    }
}
