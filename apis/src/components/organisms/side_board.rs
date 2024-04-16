use crate::{
    components::{
        atoms::undo_button::UndoButton,
        molecules::{analysis_and_download::AnalysisAndDownload, control_buttons::ControlButtons},
        organisms::{
            chat::ChatWindow,
            history::History,
            reserve::{Alignment, Reserve},
        },
    },
    providers::{auth_context::AuthContext, game_state::GameStateSignal},
};
use hive_lib::color::Color;
use leptos::*;

#[derive(Clone, PartialEq)]
enum SideboardTabView {
    Reserve,
    History,
    Chat,
}

#[component]
pub fn SideboardTabs(
    player_is_black: Memo<bool>,
    #[prop(optional)] analysis: bool,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let mut game_state_signal = expect_context::<GameStateSignal>();
    let tab_view = RwSignal::new(SideboardTabView::Reserve);
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };

    let show_buttons = move || {
        user().map_or(false, |user| {
            let game_state = game_state_signal.signal.get();
            Some(user.id) == game_state.black_id || Some(user.id) == game_state.white_id
        }) && !analysis
    };

    let button_color = move |button_view: SideboardTabView| {
        if tab_view() == button_view {
            "bg-slate-400"
        } else {
            "bg-inherit"
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
    let row_start = if analysis {
        "row-start-1"
    } else {
        "row-start-2"
    };

    view! {
        <div class=format!(
            "h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 {row_start} {extend_tw_classes}",
        )>
            <div class="z-10 border-b-2 border-black dark:border-white flex justify-between [&>*]:grow sticky top-0 bg-inherit">

                <button
                    class=move || {
                        format!(
                            "transform transition-transform duration-300 active:scale-95 hover:bg-blue-300 {}",
                            button_color(SideboardTabView::Reserve),
                        )
                    }

                    on:click=move |_| {
                        game_state_signal.view_game();
                        tab_view.set(SideboardTabView::Reserve);
                    }
                >

                    "Game"
                </button>

                <button
                    class=move || {
                        format!(
                            "transform transition-transform duration-300 active:scale-95 hover:bg-blue-300 {}",
                            button_color(SideboardTabView::History),
                        )
                    }

                    on:click=move |_| {
                        game_state_signal.view_history();
                        tab_view.set(SideboardTabView::History);
                    }
                >

                    "History"
                </button>
                <Show when=move || !analysis>
                    <button
                        class=move || {
                            format!(
                                "transform transition-transform duration-300 active:scale-95 hover:bg-blue-300 {}",
                                button_color(SideboardTabView::Chat),
                            )
                        }

                        on:click=move |_| {
                            tab_view.set(SideboardTabView::Chat);
                        }
                    >

                        "Chat"
                    </button>
                </Show>
            </div>

            {move || match tab_view() {
                SideboardTabView::Reserve => {
                    view! {
                        <div class="flex flex-col h-full">
                            <Reserve color=top_color alignment=Alignment::DoubleRow/>
                            <div class="flex justify-center flex-row-reverse items-center">
                                <Show
                                    when=move || !analysis
                                    fallback=move || {
                                        view! { <UndoButton/> }
                                    }
                                >

                                    <AnalysisAndDownload/>
                                </Show>
                                <Show when=show_buttons>
                                    <ControlButtons/>
                                </Show>
                            </div>
                            <Reserve color=bottom_color alignment=Alignment::DoubleRow/>
                        </div>
                    }
                        .into_view()
                }
                SideboardTabView::History => view! { <History/> }.into_view(),
                SideboardTabView::Chat => {
                    view! {
                        <div class="h-[95%]">
                            <ChatWindow/>
                        </div>
                    }
                        .into_view()
                }
            }}

        </div>
    }
}
