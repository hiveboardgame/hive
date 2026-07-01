use crate::{
    components::organisms::{
        analysis::{
            atoms::{CollapsibleMove, HistoryMove},
            AnalysisHistoryControls,
            DownloadTree,
            LoadTree,
        },
        reserve::{Alignment, Reserve},
    },
    hiveground::HivegroundInteraction,
    providers::analysis::{AnalysisSignal, AnalysisTree, TreeNode},
};
use hudsoni::{Color, State};
use leptos::prelude::*;
use std::{cmp::Ordering, collections::HashMap};
use tree_ds::prelude::*;

fn action_button_class() -> String {
    "ui-button ui-button-secondary ui-button-sm min-h-9 w-full whitespace-normal px-2 py-1 text-center text-xs leading-tight"
        .to_string()
}

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
        let is_start_node =
            AnalysisTree::is_start_node_id(node_id) && node.get_value().ok().flatten().is_none();

        content = match children_ids.len() {
            // The synthetic start node is only useful in the list when it owns
            // alternate move-1 branches; otherwise keep it out of the rendered history.
            0 | 1 if is_start_node => content,

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
                    view! { <HistoryMove current_path node has_children=false /> }.into_any()
                } else {
                    view! { <div>"Invalid node"</div> }.into_any()
                }
            }

            HistoryItem::Collapsible { node_id, inner } => {
                let maybe_node = analysis.with(|a| a.tree.get_node_by_id(node_id));

                if let Some(node) = maybe_node {
                    let inner_view = render_history_items(inner, analysis, current_path);

                    view! { <CollapsibleMove current_path node inner=inner_view /> }.into_any()
                } else {
                    view! { <div>"Invalid node"</div> }.into_any()
                }
            }
        })
        .collect_view()
        .into_any()
}

#[component]
pub fn History(
    interaction: HivegroundInteraction,
    history_state: Memo<State>,
    #[prop(optional)] mobile: bool,
    #[prop(optional)] hide_controls: bool,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().tree;
    let reserve_class =
        "flex flex-col py-1 px-2 rounded border border-black/5 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted";
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

    let has_history = Memo::new(move |_| analysis.with(|a| a.has_real_moves()));
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

    let history_model =
        Memo::new(move |_| analysis.with(|a| build_history_model(&a.tree).unwrap_or_default()));

    let viewbox_str = "-32 -40 250 120";
    view! {
        <div class="flex flex-col gap-3 min-h-0 size-full">
            <Show when=move || !hide_controls>
                <AnalysisHistoryControls scroll_on_navigate=!mobile />
            </Show>
            <Show when=move || !mobile>
                <div class=reserve_class>
                    <Reserve
                        alignment=Alignment::DoubleRow
                        color=Color::Black
                        viewbox_str
                        interaction
                        history_state
                    />
                    <Reserve
                        alignment=Alignment::DoubleRow
                        color=Color::White
                        viewbox_str
                        interaction
                        history_state
                    />
                </div>
            </Show>
            <div class="flex gap-2 items-center w-full">
                <Show when=has_history>
                    <DownloadTree />
                </Show>
                <LoadTree />
            </div>
            <div class="grid gap-2 w-full grid-cols-[repeat(auto-fit,minmax(7rem,1fr))]">
                <button on:click=move |_| promote_variation(true) class=action_button_class()>
                    "Make main line"
                </button>
                <button on:click=move |_| promote_variation(false) class=action_button_class()>
                    "Promote variation"
                </button>
            </div>
            <div class="overflow-y-auto flex-grow p-2 min-h-0 text-sm rounded border border-black/5 bg-even-light/70 dark:border-white/10 dark:bg-surface-field">
                {move || {
                    history_model
                        .with(|items| { render_history_items(items, analysis, current_path) })
                }}

            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(turn: usize, piece: &str) -> TreeNode {
        TreeNode {
            turn,
            piece: piece.to_string(),
            position: String::new(),
        }
    }

    #[test]
    fn empty_start_node_is_not_rendered() {
        let mut tree = Tree::new(Some("analysis"));
        tree.add_node(Node::new(-1, None), None).unwrap();

        assert_eq!(build_history_model(&tree).unwrap(), Vec::new());
    }

    #[test]
    fn single_line_hides_start_node() {
        let mut tree = Tree::new(Some("analysis"));
        let start = tree.add_node(Node::new(-1, None), None).unwrap();
        let move_1 = tree
            .add_node(Node::new(0, Some(node(1, "wG1"))), Some(&start))
            .unwrap();
        tree.add_node(Node::new(1, Some(node(2, "bG1"))), Some(&move_1))
            .unwrap();

        assert_eq!(
            build_history_model(&tree).unwrap(),
            vec![
                HistoryItem::Move { node_id: 0 },
                HistoryItem::Move { node_id: 1 },
            ]
        );
    }

    #[test]
    fn start_node_variations_are_collapsible_branches() {
        let mut tree = Tree::new(Some("analysis"));
        let start = tree.add_node(Node::new(-1, None), None).unwrap();
        let main_1 = tree
            .add_node(Node::new(0, Some(node(1, "wG1"))), Some(&start))
            .unwrap();
        tree.add_node(Node::new(1, Some(node(2, "bG1"))), Some(&main_1))
            .unwrap();
        let variation_1 = tree
            .add_node(Node::new(2, Some(node(1, "wM"))), Some(&start))
            .unwrap();
        tree.add_node(Node::new(3, Some(node(2, "bG1"))), Some(&variation_1))
            .unwrap();

        assert_eq!(
            build_history_model(&tree).unwrap(),
            vec![
                HistoryItem::Collapsible {
                    node_id: -1,
                    inner: vec![
                        HistoryItem::Move { node_id: 2 },
                        HistoryItem::Move { node_id: 3 },
                    ],
                },
                HistoryItem::Move { node_id: 0 },
                HistoryItem::Move { node_id: 1 },
            ]
        );
    }
}
