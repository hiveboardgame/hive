use crate::{
    components::{
        molecules::history_controls::HistoryControls,
        organisms::{chat::ChatWindow, history::History, reserve::ReserveContent},
    },
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, SimpleDestination};

#[derive(Clone, PartialEq, Copy)]
enum TabView {
    Reserve,
    History,
    Chat,
}
#[component]
fn TriggerButton(name: TabView, tab: RwSignal<TabView>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let params = use_params_map();
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
    let mut game_state = expect_context::<GameStateSignal>();
    view! {
        <div
            on:click=move |_| {
                if tab() == TabView::Chat {
                    chat.seen_messages(game_id());
                }
                if name == TabView::History {
                    game_state.view_history();
                } else if name == TabView::Reserve {
                    game_state.view_game();
                }
                tab.update(|v| *v = name);
            }

            class=move || {
                format!(
                    "flex place-content-center transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal {}",
                    if tab() == name {
                        "dark:bg-button-twilight bg-slate-400"
                    } else if name == TabView::Chat && chat.has_messages(game_id()) {
                        "bg-ladybug-red"
                    } else {
                        "bg-inherit"
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
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let tab = RwSignal::new(TabView::Reserve);
    let user = auth_context.user;
    let white_and_black = create_read_slice(game_state.signal, |gs| (gs.white_id, gs.black_id));
    let show_buttons = Signal::derive(move || {
        user().is_some_and(|user| {
            let (white_id, black_id) = white_and_black();
            Some(user.id) == black_id || Some(user.id) == white_id
        })
    });
    view! {
        <div class=format!(
            "bg-reserve-dawn dark:bg-reserve-twilight h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 row-start-2 relative {extend_tw_classes}",
        )>

            <div>
                <div class="z-10 border-b-2 border-black dark:border-white flex justify-between [&>*]:grow sticky top-0 bg-inherit">
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
                class="flex flex-col flex-grow h-full max-h-full justify-beetween"
            >
                <HistoryControls />
                <ChatWindow destination=SimpleDestination::Game />
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
