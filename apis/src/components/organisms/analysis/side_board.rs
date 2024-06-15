use crate::{
    components::{
        atoms::undo_button::UndoButton, organisms::{
            history::History,
            reserve::{Alignment, Reserve},
        }
    },
    providers::game_state::GameStateSignal,
};
use hive_lib::Color;
use leptos::*;

use leptix_primitives::components::tabs::{TabsContent, TabsList, TabsRoot, TabsTrigger};

#[component]
pub fn SideboardTabs(
    player_is_black: Memo<bool>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {    
    let top_color = Signal::derive(move || {
        if player_is_black() {
            Color::White
        } else {
            Color::Black
        }
    });
    let bottom_color = Signal::derive(move || top_color().opposite_color());
    let mut game_state = expect_context::<GameStateSignal>();
    let button_class = move || {
        "transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal data-[state=active]:dark:bg-button-twilight data-[state=active]:bg-slate-400".to_string()
    };
view! {
    <TabsRoot
        default_value="Game"
        attr:class=format!(
                "bg-reserve-dawn dark:bg-reserve-twilight h-full flex flex-col select-none col-span-2 border-x-2 border-black dark:border-white row-span-4 row-start-1 {extend_tw_classes}"
            )
    >

        <TabsList>
            <div class="z-10 border-b-2 border-black dark:border-white flex justify-between [&>*]:grow sticky top-0 bg-inherit">
                <TabsTrigger
                    value="Game"
                    attr:class=button_class
                    on:click=move |_| game_state.view_game()
                >
                    "Game"
                </TabsTrigger>
                <TabsTrigger
                    value="History"
                    attr:class=button_class
                    on:click=move |_| game_state.view_history()
                >
                    "History"
                </TabsTrigger>
            </div>
        </TabsList>
        <TabsContent value="Game" attr:class="flex flex-col h-full">
            <Reserve color=top_color alignment=Alignment::DoubleRow/>
            <div class="flex flex-row-reverse justify-center items-center">
                <UndoButton/>
            </div>
            <Reserve color=bottom_color alignment=Alignment::DoubleRow/>
        </TabsContent>
        <TabsContent value="History">
            <History/>
        </TabsContent>
    </TabsRoot>
}
}
