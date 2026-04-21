use crate::{
    components::{
        molecules::{
            game_thread_toggle::{GameThreadToggle, GameThreadToggleSize},
            history_controls::HistoryControls,
        },
        organisms::{chat::GameChatWindow, history::History, reserve::ReserveContent},
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
use shared_types::{GameChatCapabilities, GameId, GameThread};

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
    let unread = Signal::derive(move || chat.unread_count_for_game(&game_id()));
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
                let has_unread = name == TabView::Chat && unread.get() > 0;
                format!(
                    "flex place-content-center transform transition-transform duration-300 active:scale-95 {}",
                    if tab() == name {
                        "dark:bg-button-twilight bg-slate-400"
                    } else if has_unread {
                        "bg-ladybug-red text-white hover:bg-red-600 dark:bg-red-600 dark:hover:bg-red-500"
                    } else {
                        "bg-inherit hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
                    },
                )
            }
        >
            {string.clone()}
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
    let game_chat_access = Signal::derive(move || {
        let is_player = user().is_some_and(|user| {
            let (white_id, black_id) = white_and_black();
            Some(user.id) == black_id || Some(user.id) == white_id
        });
        let finished = game_state
            .signal
            .with(|gs| gs.game_response.as_ref().is_some_and(|gr| gr.finished));
        GameChatCapabilities::new(is_player, finished)
    });
    let show_buttons = Signal::derive(move || game_chat_access.get().can_toggle_embedded_threads());
    let selected_game_thread = RwSignal::new(GameThread::Players);
    let explicit_game_thread =
        Signal::derive(move || show_buttons().then_some(selected_game_thread.get()));
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
            <TabsContent value=TabView::Reserve class="flex flex-col flex-1 min-h-0" tab>
                <ReserveContent player_color show_buttons />
            </TabsContent>
            <TabsContent value=TabView::History class="flex-1 min-h-0" tab>
                <History />
            </TabsContent>
            <TabsContent
                tab
                value=TabView::Chat
                class="flex overflow-hidden flex-col flex-1 min-h-0"
            >
                <HistoryControls />
                <Show when=move || show_buttons()>
                    <GameThreadToggle
                        selected=selected_game_thread
                        spectators_enabled=Signal::derive(move || {
                            game_chat_access.get().can_read(GameThread::Spectators)
                        })
                        size=GameThreadToggleSize::Compact
                    />
                </Show>
                <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                    <GameChatWindow explicit_thread=explicit_game_thread />
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
