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
    const MOVES_TO_SHOW: usize = 3;
    let analysis = expect_context::<AnalysisSignal>().0;
    let moves = move || {
        let analysis = analysis();
        let mut current_node = analysis.current_node.clone();
        let mut moves = Vec::new();
        for _ in 0..MOVES_TO_SHOW {
            let first_child = if let Some(ref node) = current_node {
                let children = node.get_children_ids();
                if children.is_empty() {
                    None
                } else {
                    analysis.tree.get_node_by_id(&children[0])
                }
            } else {
                None
            };
            if let Some(node) = first_child {
                moves.push(node.get_value().unwrap());
                current_node = Some(node);
            } else {
                break;
            }
        }
        if moves.is_empty() {
            String::new()
        } else {
            let more = if moves.len() == MOVES_TO_SHOW
                && current_node.is_some_and(|n| !n.get_children_ids().is_empty())
            {
                "..."
            } else {
                ""
            };
            format!(
                "Next Moves: {}{more}",
                moves
                    .into_iter()
                    .map(|m| format!("{}. {} {}", m.turn, m.piece, m.position))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    };
    view! {
        <div class=extend_tw_classes>
            <div class="flex gap-1 items-center">{moves}</div>
        </div>
    }
}
