use crate::{
    common::MoveConfirm,
    components::{
        layouts::base_layout::OrientationSignal,
        organisms::{
            analysis::{
                AnalysisSignal, AnalysisTree, HistoryButton, HistoryNavigation, SideboardTabs,
                ToggleStates, UndoButton,
            },
            board::Board,
            reserve::{Alignment, Reserve},
        },
    },
    pages::play::{CurrentConfirm, TargetStack},
    providers::game_state::GameStateSignal,
};
use hive_lib::Color;
use leptos::*;
use std::collections::HashSet;

#[component]
pub fn Analysis(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    provide_context(AnalysisSignal(RwSignal::new(Some(
        AnalysisTree::from_state(expect_context::<GameStateSignal>()).unwrap_or_default(),
    ))));
    provide_context(ToggleStates(RwSignal::new(HashSet::new())));
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    let is_tall = expect_context::<OrientationSignal>().is_tall;
    let parent_container_style = move || {
        if is_tall() {
            "flex flex-col"
        } else {
            "grid grid-cols-board-xs sm:grid-cols-board-sm lg:grid-cols-board-lg xxl:grid-cols-board-xxl grid-rows-6 pr-1"
        }
    };
    let player_color = Memo::new(move |_| Color::White);
    let bottom_color = Color::Black;
    let top_color = Color::White;

    view! {
        <div class=move || {
            format!(
                "max-h-[100dvh] min-h-[100dvh] pt-10 bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=is_tall
                fallback=move || {
                    view! {
                        <Board/>
                        <div class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-4 row-span-4 row-start-2 border-black border-y-2 dark:border-white">
                            <SideboardTabs player_color/>
                        </div>
                    }
                }
            >

                <div class="flex flex-col flex-grow h-full min-h-0">
                    <div class="flex flex-col flex-grow shrink">
                        <div class="flex justify-between h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=top_color/>
                        </div>
                    </div>
                    <Board overwrite_tw_classes="flex grow min-h-0"/>
                    <div class="flex flex-col flex-grow shrink">
                        <div class="flex justify-between h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=bottom_color/>
                            <UndoButton/>
                        </div>
                        <div class="grid grid-cols-2 gap-8">
                            <HistoryButton action=HistoryNavigation::Previous/>
                            <HistoryButton action=HistoryNavigation::Next/>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
