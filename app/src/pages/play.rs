use crate::organisms::header::Header;
use crate::organisms::{
    board::Board,
    reserve::{Orientation, Reserve},
};
use hive_lib::color::Color;
use leptos::*;

#[component]
pub fn PlayPage(cx: Scope) -> impl IntoView {
    // provide_context(cx, create_rw_signal(cx, GameState::new(cx)));

    view! { cx,
        <div class="h-screen"><Header/>
        <div class="grid grid-cols-10 h-[89vh] items-stretch w-screen overflow-hidden">
                <div class="col-start-1 col-span-1"><Reserve color=Color::White orientation=Orientation::Vertical/></div>
                <Board />
                <div class="col-start-10 col-span-1"><Reserve color=Color::Black orientation=Orientation::Vertical/></div>
        </div>
        </div>
    }
}
