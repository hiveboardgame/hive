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
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use hive_lib::Color;
use leptos::*;
use leptos_router::use_location;
use shared_types::SimpleDestination;

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
    let chat = expect_context::<Chat>();
    let tab_view = RwSignal::new(SideboardTabView::Reserve);
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    // On navigation switch to reserve
    create_effect(move |_| {
        let location = use_location();
        let _ = (location.pathname)();
        tab_view.set(SideboardTabView::Reserve);
    });
    let show_buttons = move || {
        user().map_or(false, |user| {
            let game_state = game_state_signal.signal.get();
            Some(user.id) == game_state.black_id || Some(user.id) == game_state.white_id
        }) && !analysis
    };

    let button_color = move |button_view: SideboardTabView| {
        if tab_view() == button_view {
            "dark:bg-button-twilight bg-slate-400"
        } else {
            "bg-inherit"
        }
    };
    let button_color_chat = move || {
        let chat_view = SideboardTabView::Chat;
        if tab_view() == chat_view {
            button_color(chat_view)
        } else if chat.has_messages() {
            "bg-ladybug-red"
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
            "bg-reserve-dawn dark:bg-reserve-twilight h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 {row_start} {extend_tw_classes}",
        )>
            <div class="z-10 border-b-2 border-black dark:border-white flex justify-between [&>*]:grow sticky top-0 bg-inherit">

                <button
                    class=move || {
                        format!(
                            "transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal {}",
                            button_color(SideboardTabView::Reserve),
                        )
                    }

                    on:click=move |_| {
                        batch(move || {
                            game_state_signal.view_game();
                            if tab_view() == SideboardTabView::Chat {
                                chat.seen_messages();
                            }
                            tab_view.set(SideboardTabView::Reserve);
                        });
                    }
                >

                    "Game"
                </button>

                <button
                    class=move || {
                        format!(
                            "transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal {}",
                            button_color(SideboardTabView::History),
                        )
                    }

                    on:click=move |_| {
                        batch(move || {
                            game_state_signal.view_history();
                            if tab_view() == SideboardTabView::Chat {
                                chat.seen_messages();
                            }
                            tab_view.set(SideboardTabView::History);
                        });
                    }
                >

                    "History"
                </button>
                <Show when=move || !analysis>
                    <button
                        class=move || {
                            format!(
                                "transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal {}",
                                button_color_chat(),
                            )
                        }

                        on:click=move |_| {
                            batch(move || {
                                tab_view.set(SideboardTabView::Chat);
                                chat.seen_messages();
                            });
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
                            <div class="flex flex-row-reverse justify-center items-center">
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
                            <ChatWindow destination=SimpleDestination::Game/>
                        </div>
                    }
                        .into_view()
                }
            }}

        </div>
    }
}
