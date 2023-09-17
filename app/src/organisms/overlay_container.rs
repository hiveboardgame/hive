use crate::organisms::reserve::{Orientation, Reserve};
use hive_lib::color::Color;
use leptos::*;

#[component]
pub fn OverlayTabs(cx: Scope) -> impl IntoView {
    let (active_history, set_active_history) = create_signal(cx, false);
    let button_color = move || {
        if active_history.get() {
            ("bg-inherit", "bg-slate-400")
        } else {
            ("bg-slate-400", "bg-inherit")
        }
    };

    view! { cx,
        <div
        class ="select-none">
            <div class="flex justify-around">
            <button class=move || format!("grow hover:bg-blue-300 {}", button_color().0) on:click=move |_| {

                set_active_history(false);
            }>

                "Reserve"
            </button>
            <button class=move || format!("grow hover:bg-blue-300 {}", button_color().1) on:click=move |_| {
                set_active_history(true);
            }>

                "History"
            </button>
            </div>
            <Show
                when=move || active_history()
                fallback=|cx| {
                    view! { cx,
                        <div class="">
                            <Reserve color=Color::White orientation=Orientation::Horizontal/>
                            <Reserve color=Color::Black orientation=Orientation::Horizontal/>
                        </div>
                    }
                }
            >
            History
            </Show>
        </div>
    }
}
