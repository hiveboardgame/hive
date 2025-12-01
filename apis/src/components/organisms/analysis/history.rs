use crate::components::organisms::analysis::atoms::{
    CollapsibleMove, HistoryButton, HistoryMove, HistoryNavigation,
};
use crate::components::organisms::{
    analysis::{DownloadTree, LoadTree, UndoButton},
    reserve::{Alignment, Reserve},
};
use crate::providers::analysis::{AnalysisSignal, AnalysisTree, TreeNode};
use hive_lib::Color;
use leptos::{ev::keydown, html, prelude::*};
use leptos_use::{use_event_listener, use_window};
use std::cmp::Ordering;
use std::collections::HashMap;
use tree_ds::prelude::*;

const BTN_CLASS: &str = "flex z-20 justify-center items-center m-1 w-44 h-10 text-white rounded-sm transition-transform duration-300 aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95";

#[derive(Clone, Debug, PartialEq)]
enum HistoryItem {
    /// A plain history row: <HistoryMove ... />
    Move { node_id: i32 },

    /// A collapsible row:
    /// <CollapsibleMove ... inner= ... >
    Collapsible {
        node_id: i32,
        inner: Vec<HistoryItem>,
    },
}

fn build_history_model(tree: &Tree<i32, TreeNode>) -> Option<Vec<HistoryItem>> {
    let root = tree.get_root_node()?;
    let root_id = root.get_node_id().ok()?;
    // Post-order traversal ensures children are processed before their parents.
    let node_order = tree.traverse(&root_id, TraversalStrategy::PostOrder).ok()?;

    // At each step this holds the rendered subtree for the current branch.
    let mut content: Vec<HistoryItem> = Vec::new();

    // Branch root id -> fully built content of that branch.
    let mut branches: HashMap<i32, Vec<HistoryItem>> = HashMap::new();

    for node_id in node_order {
        let node = tree.get_node_by_id(&node_id)?;
        let children_ids = node.get_children_ids().ok().unwrap_or_default();
        let siblings_ids = tree
            .get_sibling_ids(&node_id, true)
            .ok()
            .unwrap_or_default();

        let parent_degree = siblings_ids.len();
        let is_main_variation = siblings_ids.first().is_some_and(|first| *first == node_id);

        content = match children_ids.len() {
            // Multiple children: put secondary variations inside a collapsible,
            // then continue the main line inline.
            n if n > 1 => {
                // Collect full branch contents for secondary variations.
                let mut secondary_variations: Vec<HistoryItem> = Vec::new();
                for child_id in children_ids.iter().skip(1) {
                    if let Some(mut branch) = branches.remove(child_id) {
                        secondary_variations.append(&mut branch);
                    }
                }

                if let Some(first_child) = children_ids.first() {
                    // Main line content for the first child.
                    let mut main_branch = branches.remove(first_child).unwrap_or_default();

                    // Parent node is rendered as a collapsible, with the secondary
                    // variations as its inner content, then the main line inline.
                    let mut new_content = Vec::with_capacity(1 + main_branch.len());
                    new_content.push(HistoryItem::Collapsible {
                        node_id,
                        inner: secondary_variations,
                    });
                    new_content.append(&mut main_branch);
                    new_content
                } else {
                    content
                }
            }

            // Single child in a non‑main variation where the parent has >2 children:
            // wrap it in a collapsible for aesthetics.
            1 if parent_degree > 2 && !is_main_variation => {
                let inner = content;
                vec![HistoryItem::Collapsible { node_id, inner }]
            }

            // Default case: a plain HistoryMove followed by the already‑built content.
            _ => {
                let mut new_content = Vec::with_capacity(1 + content.len());
                new_content.push(HistoryItem::Move { node_id });
                new_content.extend(content);
                new_content
            }
        };

        // Start of a new branch: store its content and reset current content
        // so siblings/parent can consume it later.
        if parent_degree > 1 {
            branches.insert(node_id, std::mem::take(&mut content));
        }
    }

    debug_assert!(branches.is_empty());
    Some(content)
}

fn render_history_items(
    items: &[HistoryItem],
    analysis: RwSignal<AnalysisTree>,
    current_path: Memo<Vec<i32>>,
) -> AnyView {
    items
        .iter()
        .map(|item| match item {
            HistoryItem::Move { node_id } => {
                let maybe_node = analysis.with(|a| a.tree.get_node_by_id(node_id));

                if let Some(node) = maybe_node {
                    // Plain row: children render via their own entries, so has_children=false.
                    view! { <HistoryMove current_path node has_children=false /> }
                    .into_any()
                } else {
                    view! { <div>"Invalid node"</div> }.into_any()
                }
            }

            HistoryItem::Collapsible { node_id, inner } => {
                let maybe_node = analysis.with(|a| a.tree.get_node_by_id(node_id));

                if let Some(node) = maybe_node {
                    let inner_view = render_history_items(inner, analysis, current_path);

                    view! { <CollapsibleMove current_path node inner=inner_view /> }
                    .into_any()
                } else {
                    view! { <div>"Invalid node"</div> }.into_any()
                }
            }
        })
        .collect_view()
        .into_any()
}

#[component]
pub fn History(#[prop(optional)] mobile: bool) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
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

    let has_history = Memo::new(move |_| analysis.with(|a| a.tree.get_root_node().is_some()));
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

    let history_model =
        Memo::new(move |_| analysis.with(|a| build_history_model(&a.tree).unwrap_or_default()));

    let viewbox_str = "-32 -40 250 120";
    view! {
        <div class="flex flex-col size-full">
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
                <Show when=has_history>
                    <DownloadTree />
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
            <div class="overflow-y-auto p-1">
                {move || {
                    history_model
                        .with(|items| { render_history_items(items, analysis, current_path) })
                }}

            </div>
        </div>
    }
}
