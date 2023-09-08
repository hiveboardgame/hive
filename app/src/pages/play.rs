use crate::{
    organisms::{board::Board, reserve::Reserve},
};
use hive_lib::color::Color;
use leptos::*;

#[component]
pub fn Play(cx: Scope) -> impl IntoView {
    // provide_context(cx, create_rw_signal(cx, GameState::new(cx)));

    view! { cx,
        <div class="row" style="width: 99vw; height: 89vh; display:flex;">
                <Reserve color=Color::White/>
                <Board />
                <Reserve color=Color::Black/>
        </div>
    }
}
