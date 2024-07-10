use std::collections::HashMap;

use crate::components::organisms::analysis::atoms::{
    CollapsibleMove, HistoryButton, HistoryMove, HistoryNavigation,
};
use crate::components::organisms::{
    analysis::{AnalysisSignal, DownloadTree, LoadTree, UndoButton},
    reserve::{Alignment, Reserve},
};
use hive_lib::Color;
use leptos::{ev::keydown, *};
use leptos_use::{use_event_listener, use_window};
use tree_ds::prelude::*;

#[component]
pub fn History(#[prop(optional)] mobile: bool) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let prev_button = create_node_ref::<html::Button>();
    let next_button = create_node_ref::<html::Button>();
    let window = use_window();
    let active = Signal::derive(move || {
        window
            .as_ref()?
            .document()?
            .query_selector(".bg-orange-twilight")
            .ok()?
    });
    create_effect(move |_| {
        _ = use_event_listener(document().body(), keydown, move |evt| {
            if evt.key() == "ArrowLeft" {
                evt.prevent_default();
                if let Some(prev) = prev_button.get_untracked() {
                    prev.click()
                }
            } else if evt.key() == "ArrowRight" {
                evt.prevent_default();
                if let Some(next) = next_button.get_untracked() {
                    next.click()
                }
            }
        });
    });

    let focus = if mobile {
        None
    } else {
        Some(Callback::new(move |()| {
            if let Some(elem) = active.get_untracked() {
                elem.scroll_into_view_with_bool(false);
            }
        }))
    };
    let walk_tree = move || {
        let tree = analysis.get().unwrap().tree;
        let root = tree.get_root_node()?;
        //Post order traversal ensures all children are processed before their parents
        let node_order = tree
            .traverse(TraversalStrategy::PostOrder, &root.get_node_id())
            .ok()?;
        let mut content = Fragment::new(vec![]);
        let mut branches = HashMap::<i32, Fragment>::new();
        for node_id in node_order {
            let node = tree.get_node_by_id(&node_id)?;
            let children_ids = node.get_children_ids();
            let siblings_ids = tree.get_sibling_ids(&node_id, true).ok()?;
            let parent_deg = siblings_ids.len();
            let not_first_sibling = siblings_ids.first().map_or(true, |s| *s != node_id);
            content = if children_ids.len() > 1 {
                /* == More than one child ==
                gather all children but the first (secondary variations)
                and place them inside a collapsible
                then place the first child (main variation) at the same level as the parent
                */
                let inner = store_value(
                    children_ids
                        .iter()
                        .skip(1)
                        .map(|c| branches.remove(c))
                        .collect::<Vec<_>>(),
                );
                view! {
                    <CollapsibleMove node>{inner}</CollapsibleMove>
                    {branches.remove(&children_ids[0])}
                }
            } else if parent_deg > 2 && not_first_sibling && children_ids.len() == 1 {
                /* We make a colapsible for nodes with one child
                for aesthetic reasons, to hide its content.
                it must be that parent has already a "seccondary variation"
                (else a toggle would not be needed)
                and this must not be the "main variation" (first child)
                */
                let static_cont = store_value(content);
                view! {
                    <>
                        <CollapsibleMove node>{static_cont}</CollapsibleMove>
                    </>
                }
            } else {
                /* All other nodes are placed at the same level as the parent
                in a regular HistoryMove node */
                view! {
                    <HistoryMove node/>
                    {content}
                }
            };
            /* We are start of a new branch so clear the content
            to process either the next sibling or tho parent */
            if parent_deg > 1 {
                //save the branch only when its the start
                branches.insert(node_id, content.clone());
                content = Fragment::new(vec![]);
            }
        }
        //all branches are processed
        debug_assert!(branches.is_empty());
        Some(content)
    };
    let viewbox_str = "-32 -40 250 120";
    view! {
        <div class="flex flex-col w-full h-full">
            <div class="flex gap-1 min-h-0 [&>*]:grow">
                <HistoryButton
                    node_ref=prev_button
                    action=HistoryNavigation::Previous
                    post_action=focus
                />
                <HistoryButton
                    node_ref=next_button
                    action=HistoryNavigation::Next
                    post_action=focus
                />
                <UndoButton/>
            </div>
            <Show when=move || !mobile>
                <div class="flex flex-col p-4">
                    <Reserve alignment=Alignment::DoubleRow color=Color::White viewbox_str/>
                    <Reserve alignment=Alignment::DoubleRow color=Color::Black viewbox_str/>
                </div>
            </Show>
            <div class="flex justify-between w-full">
                <Show when=move || walk_tree().is_some()>
                    <DownloadTree tree=analysis.get().unwrap()/>
                </Show>
                <LoadTree/>
            </div>
            <div class="overflow-y-auto p-1">{walk_tree}</div>
        </div>
    }
}
