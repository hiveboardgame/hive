use crate::{
    common::game_state::{GameStateSignal, View},
    organisms::{
        history::History,
        reserve::{Orientation, Reserve},
    },
};
use hive_lib::color::Color;
use leptos::*;

#[component]
pub fn OverlayTabs(cx: Scope) -> impl IntoView {
    let game_state_signal = use_context::<RwSignal<GameStateSignal>>(cx)
        .expect("there to be a `GameState` signal provided");

    let button_color = move || {
        if let View::History = game_state_signal.get().signal.get().view {
            ("bg-inherit", "bg-slate-400")
        } else {
            ("bg-slate-400", "bg-inherit")
        }
    };

    view! { cx,
        <div class="select-none w-full h-full">
            <div class="grid grid-cols-2 gap-1">
                <button
                    class=move || format!("hover:bg-blue-300 {}", button_color().0)
                    on:click=move |_| {
                        game_state_signal.get().view_game();
                    }
                >

                    "Reserve"
                </button>

                <button
                    class=move || format!("hover:bg-blue-300 {}", button_color().1)
                    on:click=move |_| {
                        game_state_signal.get().view_history();
                    }
                >

                    "History"
                </button>

            </div>
            <Show
                when=move || View::History == game_state_signal.get().signal.get().view
                fallback=|cx| {
                    view! { cx,
                        <div class="">
                            <Reserve color=Color::White orientation=Orientation::Horizontal/>
                            <Reserve color=Color::Black orientation=Orientation::Horizontal/>
                        </div>
                    }
                }
            >

                <History/>
            </Show>
        </div>
    }
}