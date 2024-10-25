use crate::{common::HexStack, components::molecules::simple_hex_stack::SimpleHexStack};
use hive_lib::{GameType, Position, State};
use leptos::*;

#[component]
pub fn PreviewTiles() -> impl IntoView {
    let moves = "wA1; bG1 wA1-; wA2 /wA1; bG2 bG1\\; wA3 -wA1; bG3 bG1-";
    let state = State::new_from_str(moves, &GameType::MLP.to_string()).unwrap();
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

    let pieces = move || {
        thumbnail_pieces()
            .into_iter()
            .map(|hs| {
                view! { <SimpleHexStack hex_stack=hs /> }
            })
            .collect_view()
    };

    view! {
        <div class="flex relative flex-col items-center mx-1 my-2 w-72 h-36 rounded sm:h-40 sm:w-80 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <svg
                viewBox="1159 695 225 100"
                class="w-full h-full touch-none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <g>{pieces}</g>
            </svg>
        </div>
    }
}
