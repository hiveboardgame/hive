use crate::{
    components::{
        molecules::control_buttons::ControlButtons,
        organisms::{
            history::History,
            reserve::{Alignment, Reserve},
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
pub fn SideboardTabs(
    player_is_black: Memo<bool>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let mut game_state_signal = expect_context::<GameStateSignal>();

    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };

    let show_buttons = move || {
        user().map_or(false, |user| {
            let game_state = game_state_signal.signal.get();
            Some(user.id) == game_state.black_id || Some(user.id) == game_state.white_id
        })
    };

    let button_color = move || {
        if let View::History = (game_state_signal.signal)().view {
            ("bg-inherit", "bg-slate-400")
        } else {
            ("bg-slate-400", "bg-inherit")
        }
    };
    let top_color = Signal::derive(move || {
        if player_is_black() {
            Color::White
        } else {
            Color::Black
        }
    });
    let bottom_color = Signal::derive(move || top_color().opposite_color());

    view! {
        <div class=format!(
            "select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 row-start-2 {extend_tw_classes}",
        )>
            <div class="z-10 border-b-2 border-black dark:border-white grid grid-cols-2 sticky top-0 bg-inherit">
                <button
                    class=move || { format!("duration-300 hover:bg-blue-300 {}", button_color().0) }

                    on:click=move |_| {
                        game_state_signal.view_game();
                    }
                >

                    "Game"
                </button>

                <button
                    class=move || { format!("duration-300 hover:bg-blue-300 {}", button_color().1) }

                    on:click=move |_| {
                        game_state_signal.view_history();
                    }
                >

                    "History"
                </button>
            </div>
            <div class="h-full">
                <Show
                    when=move || View::History == (game_state_signal.signal)().view
                    fallback=move || {
                        view! {
                            <div class="grid h-[95%]">
                                <Reserve color=top_color alignment=Alignment::DoubleRow/>
                                <Show when=show_buttons>
                                    <ControlButtons/>
                                </Show>
                                <Reserve color=bottom_color alignment=Alignment::DoubleRow/>
                            </div>
                        }
                    }
                >

                    <History/>
                </Show>
            </div>
        </div>
    }
}
