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
    pages::play::CurrentConfirm,
    providers::{
        analysis::{AnalysisSignal, AnalysisTree, TreeNode},
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
    let game_state = expect_context::<GameStateSignal>();
    provide_context(AnalysisSignal(RwSignal::new(LocalStorage::wrap(
        AnalysisTree::from_state(game_state).unwrap_or_default(),
    ))));
    provide_context(ToggleStates(RwSignal::new(HashSet::new())));
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    let parent_container_style = move || {
        if vertical() {
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
                when=vertical
                fallback=move || {
                    view! {
                        <AnalysisInfo extend_tw_classes="absolute pl-4 pt-2 bg-board-dawn dark:bg-board-twilight" />
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
                    <AnalysisInfo />
                    <Board />
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

#[component]
fn AnalysisInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let moves = move || {
        let tree = &analysis().tree;
        let current_node = analysis().current_node.clone();
        let mut moves = Vec::new();
        let sibling_nodes = current_node
            .and_then(|n| tree.get_sibling_ids(&n.get_node_id(), false).ok())
            .map_or(Vec::new(), |s| {
                s.iter()
                    .filter_map(|id| tree.get_node_by_id(id))
                    .collect::<Vec<_>>()
            });
        for s in sibling_nodes {
            let TreeNode {
                turn,
                piece,
                position,
            } = s.get_value().unwrap();
            moves.push(
                    view! {
                        <div
                            class="underline cursor-pointer no-link-style hover:text-pillbug-teal active:scale-95"
                            on:click=move |_| {
                                analysis
                                    .update(|a| {
                                        a.update_node(s.get_node_id(), Some(game_state));
                                    })
                            }
                        >
                            {format!("{}. {} {}", turn, piece, position)}
                        </div>
                    }
                );
        }
        moves.collect_view()
    };
    view! {
        <div class=extend_tw_classes>
            <div class="flex gap-1 items-center">
                <b>"Other Lines Explored: "</b>
                {moves}
            </div>
        </div>
    }
}
