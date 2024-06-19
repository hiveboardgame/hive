use crate::{
    components::{
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
use leptix_primitives::components::tabs::{TabsContent, TabsList, TabsRoot, TabsTrigger};
use leptos::*;
use shared_types::SimpleDestination;
#[derive(Clone, PartialEq, Copy)]
enum TabView {
    Reserve,
    History,
    Chat,
}
#[component]
fn TriggerButton(name: TabView, tab: RwSignal<TabView>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let string = match name {
        TabView::Reserve => "Game".to_string(),
        TabView::History => "History".to_string(),
        TabView::Chat => "Chat".to_string(),
    };
    let mut game_state = expect_context::<GameStateSignal>();
    view! {
        <TabsTrigger
            on:click=move |_| {
                if tab() == TabView::Chat {
                    chat.seen_messages();
                }
                if name == TabView::History {
                    game_state.view_history();
                } else if name == TabView::Reserve {
                    game_state.view_game();
                }
                tab.update(|v| *v = name);
            }

            attr:class=move || {
                format!(
                    "transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal {}",
                    if tab() == name {
                        "dark:bg-button-twilight bg-slate-400"
                    } else if name == TabView::Chat && chat.has_messages() {
                        "bg-ladybug-red"
                    } else {
                        "bg-inherit"
                    },
                )
            }

            value=string.clone()
        >
            {string.clone()}
        </TabsTrigger>
    }
}

#[component]
pub fn SideboardTabs(
    player_is_black: Memo<bool>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let tab = RwSignal::new(TabView::Reserve);
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    let white_and_black = create_read_slice(game_state.signal, |gs| (gs.white_id, gs.black_id));

    let show_buttons = move || {
        user().map_or(false, |user| {
            let (white_id, black_id) = white_and_black();
            Some(user.id) == black_id || Some(user.id) == white_id
        })
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
        <TabsRoot
            default_value="Game"
            attr:class=format!(
                "bg-reserve-dawn dark:bg-reserve-twilight h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 row-start-2 {extend_tw_classes}",
            )
        >

            <TabsList>
                <div class="z-10 border-b-2 border-black dark:border-white flex justify-between [&>*]:grow sticky top-0 bg-inherit">
                    <TriggerButton name=TabView::Reserve tab/>
                    <TriggerButton name=TabView::History tab/>
                    <TriggerButton name=TabView::Chat tab/>
                </div>
            </TabsList>
            <TabsContent value="Game" attr:class="flex flex-col h-full">
                <Reserve color=top_color alignment=Alignment::DoubleRow/>
                <div class="flex flex-row-reverse justify-center items-center">
                    <AnalysisAndDownload/>
                    <Show when=show_buttons>
                        <ControlButtons/>
                    </Show>
                </div>
                <Reserve color=bottom_color alignment=Alignment::DoubleRow/>
            </TabsContent>
            <TabsContent value="History" attr:class="h-full">
                <History/>
            </TabsContent>
            <TabsContent value="Chat" attr:class="h-[95%]">
                <ChatWindow destination=SimpleDestination::Game/>
            </TabsContent>
        </TabsRoot>
    }
}
