use crate::atoms::svgs::Svgs;
use crate::molecules::board_pieces::BoardPieces;

use leptos::*;

#[component]
pub fn Board(cx: Scope) -> impl IntoView {
    view! { cx,
        <svg viewBox="1000 450 700 700" style="flex: 1" xmlns="http://www.w3.org/2000/svg">
            <Svgs/>
            <BoardPieces/>
        </svg>
    }
}
