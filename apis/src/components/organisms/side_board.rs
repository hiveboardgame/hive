use crate::{
    components::{
        molecules::control_buttons::ControlButtons,
        organisms::{
            history::History,
            reserve::{Orientation, Reserve},
        },
    },
    providers::{
        auth_context::AuthContext,
        game_state::{GameStateSignal, View},
    },
};
use hive_lib::color::Color;

use leptos::*;

#[component]
pub fn SideboardTabs(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let mut game_state_signal = expect_context::<GameStateSignal>();

    let auth_context = expect_context::<AuthContext>();
    let user = move || match untrack(auth_context.user) {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    let show_buttons = move || {
        user().map_or(false, |user| {
            let game_state = game_state_signal.signal.get();
            Some(user.id) == game_state.black_id || Some(user.id) == game_state.white_id
        })
    };

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

                    "Game"
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
            <div ref=div_ref class="overflow-auto h-full w-full">
                <Show
                    when=move || View::History == (game_state_signal.signal)().view
                    fallback=move || {
                        view! {
                            <div class="grid grid-rows-3 h-[90%]">
                                <div class="h-full w-full row-start-1">
                                    <Reserve
                                        color=Color::White
                                        orientation=Orientation::Horizontal
                                    />
                                </div>
                                <Show when=show_buttons>
                                    <ControlButtons/>
                                </Show>
                                <div class="h-full w-full row-start-3">
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
