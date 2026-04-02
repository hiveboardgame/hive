use crate::{
    components::{
        molecules::history_controls::HistoryControls,
        organisms::{chat::ChatWindow, history::History, reserve::ReserveContent},
    },
    providers::{
        chat::Chat,
        game_state::{GameStateSignal, View},
        AuthContext,
    },
};
use hive_lib::Color;
use leptos::{prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_router::{
    hooks::{query_signal_with_options, use_params_map},
    location::State,
    NavigateOptions,
};
use shared_types::{GameId, SimpleDestination};

#[derive(Clone, PartialEq, Copy)]
pub enum TabView {
    Reserve,
    History,
    Chat,
}
#[component]
fn TriggerButton(name: TabView, tab: RwSignal<TabView>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let params = use_params_map();
    let (_move, set_move) = move_query_signal();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let string = match name {
        TabView::Reserve => "Game".to_string(),
        TabView::History => "History".to_string(),
        TabView::Chat => "Chat".to_string(),
    };
    let unread = move || {
        let _ = chat.unread_counts.get();
        chat.unread_count_for_game(&game_id())
    };
    let mut game_state = expect_context::<GameStateSignal>();
    view! {
        <div
            on:click=move |_| {
                if name == TabView::Chat {
                    chat.seen_messages(game_id());
                    let is_game_view = game_state.signal.with_untracked(|gs| gs.view == View::Game);
                    if is_game_view {
                        set_move.set(None);
                    }
                }
                if name == TabView::History {
                    game_state.view_history();
                    let history_turn = game_state.signal.with_untracked(|gs| gs.history_turn);
                    set_move.set(history_turn.map(|v| v + 1));
                } else if name == TabView::Reserve {
                    game_state.view_game();
                    set_move.set(None);
                }
                tab.update(|v| *v = name);
            }

            class=move || {
                format!(
                    "flex place-content-center transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal {}",
                    if tab() == name {
                        "dark:bg-button-twilight bg-slate-400"
                    } else {
                        "bg-inherit"
                    },
                )
            }
        >
            {string.clone()}
            {move || (name == TabView::Chat && unread() > 0).then(|| view! {
                <span class="h-5 min-w-5 flex items-center justify-center px-1 text-xs font-bold leading-none text-white bg-ladybug-red dark:bg-red-500 rounded-full">
                    {if unread() > 99 { "99+".to_string() } else { unread().to_string() }}
                </span>
            })}
        </div>
    }
}

#[component]
pub fn SideboardTabs(
    player_color: Memo<Color>,
    tab: RwSignal<TabView>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let user = auth_context.user;
    let white_and_black = create_read_slice(game_state.signal, |gs| (gs.white_id, gs.black_id));
    let show_buttons = Signal::derive(move || {
        user().is_some_and(|user| {
            let (white_id, black_id) = white_and_black();
            Some(user.id) == black_id || Some(user.id) == white_id
        })
    });
    let game_finished = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map_or(false, |gr| gr.finished)
    });
    let game_show_players = RwSignal::new(true);
    view! {
        <div class=format!(
            "bg-reserve-dawn dark:bg-reserve-twilight h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 row-start-2 relative {extend_tw_classes}",
        )>

            <div>
                <div class="flex sticky top-0 z-10 justify-between border-b-2 border-black dark:border-white [&>*]:grow bg-inherit">
                    <TriggerButton name=TabView::Reserve tab />
                    <TriggerButton name=TabView::History tab />
                    <TriggerButton name=TabView::Chat tab />
                </div>
            </div>
            <TabsContent value=TabView::Reserve class="flex flex-col h-full" tab>
                <ReserveContent player_color show_buttons />
            </TabsContent>
            <TabsContent value=TabView::History class="h-full" tab>
                <History />
            </TabsContent>
            <TabsContent
                tab
                value=TabView::Chat
                class="flex flex-col flex-grow min-h-0 h-full overflow-hidden"
            >
                <HistoryControls />
                {move || {
                    if show_buttons() {
                        let finished = game_finished();
                        view! {
                            <div class="shrink-0 flex border-b border-black/30 dark:border-white/30 p-1 gap-0.5 bg-inherit">
                                <button
                                    type="button"
                                    class=move || format!(
                                        "flex-1 px-2 py-1 text-xs font-medium rounded border border-transparent transition-colors {}",
                                        if game_show_players.get() {
                                            "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                                        } else {
                                            "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                                        }
                                    )
                                    on:click=move |_| game_show_players.set(true)
                                >
                                    "Players"
                                </button>
                                <button
                                    type="button"
                                    disabled=move || !game_finished()
                                    class=move || format!(
                                        "flex-1 px-2 py-1 text-xs font-medium rounded border border-transparent transition-colors {}",
                                        if !game_show_players.get() {
                                            "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                                        } else if finished {
                                            "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                                        } else {
                                            "bg-transparent text-gray-500 dark:text-gray-500 cursor-not-allowed"
                                        }
                                    )
                                    on:click=move |_| game_show_players.set(false)
                                >
                                    "Spectators"
                                </button>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {}.into_any()
                    }
                }}
                <div class="flex-1 min-h-0 overflow-hidden flex flex-col">
                    {move || {
                        if show_buttons() {
                            view! {
                                <ChatWindow
                                    destination=SimpleDestination::Game
                                    game_channel_override=Signal::derive(move || game_show_players.get())
                                />
                            }
                                .into_any()
                        } else {
                            view! {
                                <ChatWindow destination=SimpleDestination::Game />
                            }
                                .into_any()
                        }
                    }}
                </div>
            </TabsContent>
        </div>
    }
}

#[component]
fn TabsContent(
    tab: RwSignal<TabView>,
    value: TabView,
    class: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <Show when=move || tab() == value>
            <div class=class>{children()}</div>
        </Show>
    }
}

pub fn move_query_signal() -> (Memo<Option<usize>>, SignalSetter<Option<usize>>) {
    let nav_options = NavigateOptions {
        resolve: true,
        replace: true,
        scroll: true,
        state: State::new(None),
    };
    query_signal_with_options::<usize>("move", nav_options)
}
