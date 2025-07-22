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
            a.current_node
                .as_ref()
                .and_then(|node| node.get_node_id().ok())
                .map(|current_id| {
                    let mut path = vec![current_id];
                    if let Ok(ancestors) = a.tree.get_ancestor_ids(&current_id) {
                        path.extend(ancestors);
                    }
                    path
                })
                .unwrap_or_default()
        })
    });
    let promote_variation = move |promote_all: bool| {
        analysis.update(|a| {
            let current_path = current_path();
            let current_path = current_path
                .iter()
                .filter_map(|id| a.tree.get_node_by_id(id));
            for node in current_path {
                let Some((parent_id, current_id)) = node
                    .get_parent_id()
                    .ok()
                    .flatten()
                    .zip(node.get_node_id().ok())
                else {
                    continue;
                };

                let Some(parent) = a.tree.get_node_by_id(&parent_id) else {
                    continue;
                };
                let Ok(children) = parent.get_children_ids() else {
                    continue;
                };

                if children.first().is_some_and(|id| *id != current_id) {
                    let _ = parent.sort_children(|a, b| {
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

            // Post order traversal ensures all children are processed before their parents
            let root_id = root.get_node_id().ok()?;
            let node_order = tree.traverse(&root_id, TraversalStrategy::PostOrder).ok()?;

            let mut content = "".into_any();
            let mut branches = HashMap::<i32, AnyView>::new();

            for node_id in node_order {
                let node = tree.get_node_by_id(&node_id)?;
                let children_ids = node.get_children_ids().ok()?;
                let siblings_ids = tree.get_sibling_ids(&node_id, true).ok()?;

                let parent_degree = siblings_ids.len();
                let is_main_variation = siblings_ids.first().is_some_and(|first| *first == node_id);

                content = match children_ids.len() {
                    // Multiple children: create collapsible for secondary variations
                    n if n > 1 => {
                        /* == More than one child ==
                           gather all children but the first (secondary variations)
                           and place them inside a collapsible
                           then place the first child (main variation) at the same level as the parent
                        */
                        let secondary_variations = children_ids
                            .iter()
                            .skip(1)
                            .filter_map(|c| branches.remove(c))
                            .collect::<Vec<_>>()
                            .into_any();

                        if let Some(first_child) = children_ids.first() {
                            view! {
                                <CollapsibleMove current_path node inner=secondary_variations />
                                {branches.remove(first_child)}
                            }
                            .into_any()
                        } else {
                            content
                        }
                    }

                    // Single child in non-main variation: create collapsible for aesthetics
                    1 if parent_degree > 2 && !is_main_variation => {
                        /* We make a collapsible for nodes with one child
                           for aesthetic reasons, to hide its content.
                           it must be that parent has already a "secondary variation"
                           (else a toggle would not be needed)
                           and this must not be the "main variation" (first child)
                        */
                        view! {
                            <CollapsibleMove current_path node inner=content />
                        }
                        .into_any()
                    }

                    // Default case: regular node
                    _ => {
                        /* All other nodes are placed at the same level as the parent
                        in a regular HistoryMove node */
                        view! {
                            <HistoryMove current_path node has_children=false />
                            {content}
                        }
                        .into_any()
                    }
                };

                /* We are start of a new branch so clear the content
                to process either the next sibling or the parent */
                if parent_degree > 1 {
                    // save the branch only when its the start
                    branches.insert(node_id, content);
                    content = "".into_any();
                }
            }

            // all branches are processed
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
