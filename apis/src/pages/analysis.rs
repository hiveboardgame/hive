use crate::{
    components::{
        atoms::{
            history_button::{HistoryButton, HistoryNavigation},
            undo_button::UndoButton,
        },
        organisms::{
            board::Board,
            reserve::{Alignment, Reserve},
            side_board::SideboardTabs,
        },
    },
    pages::play::TargetStack,
    providers::game_state::GameStateSignal,
};
use hive_lib::color::Color;
use leptos::*;
use leptos_use::use_media_query;

#[component]
pub fn Analysis(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    let is_tall = use_media_query("(min-height: 100vw)");
    let parent_container_style = move || {
        if is_tall() {
            "flex flex-col"
        } else {
            "grid grid-cols-board-xs sm:grid-cols-board-sm lg:grid-cols-board-lg xxl:grid-cols-board-xxl grid-rows-6 pr-1"
        }
    };
    let player_is_black = create_memo(move |_| false);
    let go_to_game = Callback::new(move |()| {
        let mut game_state = expect_context::<GameStateSignal>();
        if game_state.signal.get_untracked().is_last_turn() {
            game_state.view_game();
        }
    });
    let bottom_color = Color::Black;
    let top_color = Color::White;

    view! {
        <div class=move || {
            format!(
                "max-h-[100dvh] min-h-[100dvh] pt-10 {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=is_tall
                fallback=move || {
                    view! {
                        <Board/>
                        <div class="grid border-y-2 border-black dark:border-white col-start-9 col-span-2 row-start-2 row-span-4 grid-cols-2 grid-rows-4">
                            <SideboardTabs player_is_black=player_is_black analysis=true/>
                        </div>
                    }
                }
            >

                <div class="flex flex-col flex-grow h-full min-h-0">
                    <div class="flex flex-col shrink flex-grow">
                        <div class="flex max-h-16 justify-between h-full">
                            <Reserve alignment=Alignment::SingleRow color=top_color/>
                        </div>
                    </div>
                    <Board overwrite_tw_classes="flex grow min-h-0"/>
                    <div class="flex flex-col shrink flex-grow">
                        <div class="flex max-h-16 justify-between h-full">
                            <Reserve alignment=Alignment::SingleRow color=bottom_color/>
                            <UndoButton/>
                        </div>
                        <div class="grid grid-cols-4 gap-8">
                            <HistoryButton action=HistoryNavigation::First/>
                            <HistoryButton action=HistoryNavigation::Previous/>
                            <HistoryButton action=HistoryNavigation::Next post_action=go_to_game/>
                            <HistoryButton action=HistoryNavigation::MobileLast/>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
