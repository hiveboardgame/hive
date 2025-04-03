use crate::{
    common::MoveConfirm,
    components::{
        layouts::base_layout::OrientationSignal,
        organisms::{
            analysis::History,
            board::Board,
            reserve::{Alignment, Reserve},
        },
    },
    pages::play::{CurrentConfirm, TargetStack},
    providers::{
        analysis::{AnalysisSignal, AnalysisTree},
        game_state::GameStateSignal,
    },
};
use hive_lib::Color;
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct ToggleStates(pub RwSignal<HashSet<i32>>);

#[component]
pub fn Analysis(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let mut game_state = expect_context::<GameStateSignal>();
    game_state.do_analysis();
    provide_context(TargetStack(RwSignal::new(None)));
    provide_context(AnalysisSignal(RwSignal::new(LocalStorage::wrap(
        AnalysisTree::from_state(game_state).unwrap_or_default(),
    ))));
    provide_context(ToggleStates(RwSignal::new(HashSet::new())));
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    let is_tall = expect_context::<OrientationSignal>().is_tall;
    let parent_container_style = move || {
        if is_tall() {
            "flex flex-col h-full"
        } else {
            "max-h-[100dvh] min-h-[100dvh] grid grid-cols-10  grid-rows-6 pr-1"
        }
    };
    let bottom_color = Color::Black;
    let top_color = Color::White;

    view! {
        <div class=move || {
            format!(
                "pt-12 bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=is_tall
                fallback=move || {
                    view! {
                        <Board />
                        <div class="flex flex-col col-span-2 row-span-6 p-1 h-full border-2 border-black select-none dark:border-white">
                            <History />
                        </div>
                    }
                }
            >

                <div class="flex flex-col h-[85dvh]">
                    <div class="flex flex-col flex-grow shrink">
                        <div class="flex justify-between h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=top_color />
                        </div>
                    </div>
                    <Board overwrite_tw_classes="flex grow min-h-0" />
                    <div class="flex flex-col flex-grow shrink">
                        <div class="flex justify-between h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=bottom_color />
                        </div>
                    </div>
                </div>
                <History mobile=true />
            </Show>
        </div>
    }
}
