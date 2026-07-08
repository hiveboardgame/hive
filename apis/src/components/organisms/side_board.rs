use crate::{
    components::{
        molecules::{
            game_thread_toggle::{
                use_embedded_game_chat_state,
                GameThreadToggle,
                GameThreadToggleSize,
            },
            history_controls::HistoryControls,
        },
        organisms::{chat::GameChatWindow, history::History, reserve::ReserveContent},
    },
    hiveground::HivegroundInteraction,
    providers::{
        chat::Chat,
        game_state::{GameStateSignal, View},
    },
};
use hive_lib::{Color, State as HiveState};
use leptos::{prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_router::{
    hooks::{query_signal_with_options, use_params_map},
    location::State,
    NavigateOptions,
};
use shared_types::{GameId, GameThread};

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
                    "ui-board-tab-trigger cursor-pointer {}",
                    if tab() == name {
                        "ui-segmented-active hover:bg-button-dawn dark:hover:bg-button-twilight"
                    } else if has_unread {
                        "ui-button-danger hover:bg-ladybug-red"
                    } else {
                        "hover:bg-blue-light/70 dark:hover:bg-pillbug-teal/15"
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
    interaction: HivegroundInteraction,
    history_state: Memo<HiveState>,
) -> impl IntoView {
    let game_chat = use_embedded_game_chat_state();
    let show_buttons = Signal::derive(move || game_chat.access.get().can_toggle_embedded_threads());
    view! {
        <div class="flex relative flex-col col-span-2 row-span-4 row-start-2 h-full min-h-0 select-none ui-board-side-panel">

            <div>
                <div class="sticky top-0 z-10 ui-board-tab-list">
                    <TriggerButton name=TabView::Reserve tab />
                    <TriggerButton name=TabView::History tab />
                    <TriggerButton name=TabView::Chat tab />
                </div>
            </div>
            <TabsContent value=TabView::Reserve class="flex flex-col h-full min-h-0" tab>
                <ReserveContent player_color show_buttons interaction history_state />
            </TabsContent>
            <TabsContent value=TabView::History class="h-full min-h-0" tab>
                <History interaction history_state />
            </TabsContent>
            <TabsContent
                tab
                value=TabView::Chat
                class="flex flex-col flex-grow justify-between h-full min-h-0 max-h-full"
            >
                <HistoryControls interaction history_state />
                <Show when=show_buttons>
                    <GameThreadToggle
                        selected=game_chat.selected_thread
                        spectators_enabled=Signal::derive(move || {
                            game_chat.access.get().can_read(GameThread::Spectators)
                        })
                        size=GameThreadToggleSize::Compact
                    />
                </Show>
                <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                    <GameChatWindow explicit_thread=game_chat.explicit_thread />
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
