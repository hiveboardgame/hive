use crate::components::organisms::analysis::atoms::{
    CollapsibleMove, HistoryButton, HistoryMove, HistoryNavigation,
};
use crate::components::organisms::{
    analysis::{DownloadTree, LoadTree, UndoButton},
    reserve::{Alignment, Reserve},
};
use crate::providers::analysis::{AnalysisSignal, AnalysisTree};
use hive_lib::Color;
use leptos::{ev::keydown, html, prelude::*};
use leptos_use::{use_event_listener, use_window};
use std::cmp::Ordering;
use std::collections::HashMap;
use tree_ds::prelude::*;

const BTN_CLASS: &str = "flex z-20 justify-center items-center m-1 w-44 h-10 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95";

#[component]
pub fn History(#[prop(optional)] mobile: bool) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let get_tree = move || {
        analysis.with(|a| {
            let out = AnalysisTree {
                current_node: a.current_node.clone(),
                tree: a.tree.clone(),
                hashes: a.hashes.clone(),
                game_type: a.game_type,
            };
            serde_json::to_string(&out).unwrap()
        })
    };
    let current_path = Memo::new(move |_| {
        analysis.with(|a| {
            let mut current_path = vec![];
            if let Some(current_node) = &a.current_node {
                let current_id = current_node.get_node_id();
                current_path.push(current_id);
                if let Ok(ancestors) = a.tree.get_ancestor_ids(&current_id) {
                    current_path.extend(ancestors);
                }
            }
            current_path
        })
    });
    let promote_variation = move |promote_all: bool| {
        analysis.update(|a| {
            let current_path = current_path();
            let current_path = current_path
                .iter()
                .filter_map(|id| a.tree.get_node_by_id(id));
            for node in current_path {
                if let Some(parent) = node
                    .get_parent_id()
                    .and_then(|id| a.tree.get_node_by_id(&id))
                {
                    let current_id = node.get_node_id();
                    if parent
                        .get_children_ids()
                        .first()
                        .is_some_and(|id| *id != current_id)
                    {
                        parent.sort_children(|a, b| {
                            if a == &current_id {
                                Ordering::Less
                            } else if b == &current_id {
                                Ordering::Greater
                            } else {
                                Ordering::Equal
                            }
                        });
                        if !promote_all {
                            break;
                        }
                    }
                }
            }
        });
    };

    let prev_button = NodeRef::<html::Button>::new();
    let next_button = NodeRef::<html::Button>::new();
    Effect::new(move |_| {
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
            let active = use_window()
                .as_ref()
                .and_then(|w| w.document())
                .and_then(|d| d.query_selector(".bg-orange-twilight").ok())
                .flatten();
            if let Some(elem) = active {
                elem.scroll_into_view_with_bool(false);
            }
        }))
    };
    let walk_tree = move || {
        analysis.with(|a| {
            let tree = &a.tree;
            let root = tree.get_root_node()?;
            //Post order traversal ensures all children are processed before their parents
            let node_order = tree
                .traverse(&root.get_node_id(), TraversalStrategy::PostOrder)
                .ok()?;
            let mut content = "".into_any();
            let mut branches = HashMap::<i32, AnyView>::new();
            for node_id in node_order {
                let node = tree.get_node_by_id(&node_id)?;
                let children_ids = node.get_children_ids();
                let siblings_ids = tree.get_sibling_ids(&node_id, true).ok()?;
                let parent_deg = siblings_ids.len();
                let not_first_sibling = siblings_ids.first().is_none_or(|s| *s != node_id);
                content = if children_ids.len() > 1 {
                    /* == More than one child ==
                    gather all children but the first (secondary variations)
                    and place them inside a collapsible
                    then place the first child (main variation) at the same level as the parent
                    */
                    let inner = children_ids
                        .iter()
                        .skip(1)
                        .map(|c| branches.remove(c))
                        .collect::<Vec<_>>()
                        .into_any();
                    view! {
                        <CollapsibleMove current_path node inner />
                        {branches.remove(&children_ids[0])}
                    }
                    .into_any()
                } else if parent_deg > 2 && not_first_sibling && children_ids.len() == 1 {
                    /* We make a colapsible for nodes with one child
                    for aesthetic reasons, to hide its content.
                    it must be that parent has already a "seccondary variation"
                    (else a toggle would not be needed)
                    and this must not be the "main variation" (first child)
                    */
                    //let static_cont = StoredValue::new(content);
                    view! { <CollapsibleMove current_path node inner=content /> }.into_any()
                } else {
                    /* All other nodes are placed at the same level as the parent
                    in a regular HistoryMove node */
                    view! {
                        <HistoryMove current_path node has_children=false />
                        {content}
                    }
                    .into_any()
                };
                /* We are start of a new branch so clear the content
                to process either the next sibling or tho parent */
                if parent_deg > 1 {
                    //save the branch only when its the start
                    branches.insert(node_id, content);
                    content = "".into_any();
                }
            }
            //all branches are processed
            debug_assert!(branches.is_empty());
            Some(content)
        })
    };
    let viewbox_str = "-32 -40 250 120";
    view! {
        <div class="flex flex-col w-full h-full">
            <div class="flex gap-1 min-h-0 [&>*]:grow">
                <HistoryButton action=HistoryNavigation::First post_action=focus />
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
                <UndoButton />
            </div>
            <Show when=move || !mobile>
                <div class="flex flex-col p-4">
                    <Reserve alignment=Alignment::DoubleRow color=Color::White viewbox_str />
                    <Reserve alignment=Alignment::DoubleRow color=Color::Black viewbox_str />

                </div>
            </Show>
            <div class="flex justify-between w-full">
                <Show when=move || walk_tree().is_some()>
                    <DownloadTree tree=get_tree() />
                </Show>
                <LoadTree />
            </div>
            <div class="flex justify-between w-full">
                <button on:click=move |_| promote_variation(true) class=BTN_CLASS>
                    "Make main line"
                </button>
                <button on:click=move |_| promote_variation(false) class=BTN_CLASS>
                    "Promote variation"
                </button>
            </div>
            <div class="overflow-y-auto p-1">{walk_tree}</div>
        </div>
    }
}
