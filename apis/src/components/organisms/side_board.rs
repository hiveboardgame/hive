use crate::{
    components::organisms::{
        history::History,
        reserve::{Orientation, Reserve},
    },
    providers::game_state::{GameStateSignal, View},
};
use hive_lib::color::Color;
use leptos::*;

#[component]
pub fn SideboardTabs(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let mut game_state_signal = expect_context::<GameStateSignal>();
    let div_ref = create_node_ref::<html::Div>();
    let button_color = move || {
        if let View::History = (game_state_signal.signal)().view {
            ("bg-inherit", "bg-slate-400")
        } else {
            ("bg-slate-400", "bg-inherit")
        }
    };

    view! {
        <div class=format!(
            "select-none w-full h-full col-start-9 col-span-2 border-2 row-span-4 row-start-2 {extend_tw_classes}",
        )>

            <div class="grid grid-cols-2 gap-1 sticky top-0 dark:bg-gray-900 bg-white">
                <button
                    class=move || format!("hover:bg-blue-300 {}", button_color().0)
                    on:click=move |_| {
                        game_state_signal.view_game();
                    }
                >

                    "Reserve"
                </button>

                <button
                    class=move || format!("hover:bg-blue-300 {}", button_color().1)
                    on:click=move |_| {
                        let parent_div = div_ref.get_untracked().expect("div to have loaded");
                        game_state_signal.view_history();
                        parent_div.set_scroll_top(parent_div.scroll_height());
                    }
                >

                    "History"
                </button>

            </div>
            <div ref=div_ref class="overflow-auto h-[90%] w-full">
                <Show
                    when=move || View::History == game_state_signal.signal.get().view
                    fallback=|| {
                        view! {
                            <div class="">
                                <div>
                                    <p>White Name 8001</p>
                                    <Reserve
                                        color=Color::White
                                        orientation=Orientation::Horizontal
                                    />
                                </div>
                                <div>
                                    <p>Black Name 9001</p>
                                    <Reserve
                                        color=Color::Black
                                        orientation=Orientation::Horizontal
                                    />
                                </div>
                            </div>
                        }
                    }
                >

                    <History parent_div=div_ref/>
                </Show>
            </div>
        </div>
    }
}

