use crate::{
    pages::analysis::ToggleStates,
    providers::{
        analysis::{AnalysisStore, AnalysisTreeStoreFields, TreeNode},
        game_state::{GameStateStore, GameStateStoreFields},
    },
};
use leptos::{html, prelude::*};
use leptos_icons::Icon;
use tree_ds::prelude::Node;

#[component]
pub fn UndoButton() -> impl IntoView {
    let analysis = expect_context::<AnalysisStore>();
    let game_state = expect_context::<GameStateStore>();
    let disabled_analysis = analysis.clone();
    let is_disabled = move || disabled_analysis.current_node().get().is_none();
    let undo = move |_| {
        let mut needs_sync = false;
        analysis.update(|a| {
            if let Some(node) = &a.current_node {
                if let Ok(node_id) = node.get_node_id() {
                    if let Ok(Some(new_current)) = node.get_parent_id() {
                        while let Some(v) = a.full_path.last() {
                            if v != &node_id {
                                a.full_path.pop();
                            }
                        }
                        a.full_path.pop();
                        a.current_node = a.tree.get_node_by_id(&new_current);
                        needs_sync = true;
                        if let Ok(tree) = a.tree.get_subtree(&node_id, None) {
                            tree.get_nodes().iter().for_each(|n| {
                                if let Ok(n_id) = n.get_node_id() {
                                    a.hashes.remove_by_right(&n_id);
                                }
                            });
                        };
                        let _ = a.tree.remove_node(
                            &node_id,
                            tree_ds::prelude::NodeRemovalStrategy::RemoveNodeAndChildren,
                        );
                    } else {
                        a.reset(game_state);
                    }
                }
            }
        });
        if needs_sync {
            analysis.sync_game_state(game_state);
        }
    };

    view! {
        <button
            class="flex justify-center place-items-center m-1 h-7 rounded-md border-2 border-cyan-500 transition-transform duration-300 active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed drop-shadow-lg dark:hover:bg-pillbug-teal dark:border-button-twilight hover:bg-pillbug-teal disabled:hover:bg-transparent"
            on:click=undo
            prop:disabled=is_disabled
        >
            <Icon icon=icondata_bi::BiUndoRegular attr:class="size-6" />
        </button>
    }
}

use leptos::leptos_dom::helpers::debounce;
#[derive(Clone)]
pub enum HistoryNavigation {
    First,
    Next,
    Previous,
}

#[component]
pub fn HistoryButton(
    action: HistoryNavigation,
    post_action: Option<Callback<()>>,
    #[prop(optional)] node_ref: Option<NodeRef<html::Button>>,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisStore>();
    let game_state = expect_context::<GameStateStore>();
    let current_node = analysis.current_node();
    let tree = analysis.tree();
    let cloned_action = action.clone();
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 m-1 h-7 rounded-md border-cyan-500 dark:border-button-twilight border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let icon = match action {
        HistoryNavigation::First => icondata_ai::AiFastBackwardFilled,
        HistoryNavigation::Next => icondata_ai::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata_ai::AiStepBackwardFilled,
    };

    let is_disabled = move || {
        if let Some(n) = current_node.get() {
            match cloned_action {
                HistoryNavigation::First => n.get_node_id().map_or(true, |node_id| {
                    tree.with(|analysis_tree| {
                        analysis_tree
                            .get_ancestor_ids(&node_id)
                            .map_or(true, |ids| ids.is_empty())
                    })
                }),
                HistoryNavigation::Next => n
                    .get_children_ids()
                    .map_or_else(|_| true, |children| children.is_empty()),
                HistoryNavigation::Previous => n.get_parent_id().map_or(true, |p| p.is_none()),
            }
        } else {
            false
        }
    };
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        let updated_node_id = current_node.with_untracked(|a| {
            a.as_ref().and_then(|n| match action {
                HistoryNavigation::First => n
                    .get_node_id()
                    .ok()
                    .and_then(|node_id| tree.with_untracked(|t| t.get_ancestor_ids(&node_id).ok()))
                    .and_then(|ids| ids.last().cloned()),
                HistoryNavigation::Next => n
                    .get_children_ids()
                    .ok()
                    .and_then(|children| children.first().cloned()),
                HistoryNavigation::Previous => n.get_parent_id().ok().flatten(),
            })
        });
        if let Some(updated_node_id) = updated_node_id {
            analysis.update_node(updated_node_id);
            analysis.sync_game_state(game_state);
        }
        if let Some(post_action) = post_action {
            post_action.run(())
        }
    });
    let _definite_node_ref = node_ref.unwrap_or_default();

    view! {
        <button
            node_ref=_definite_node_ref
            class=nav_buttons_style
            disabled=is_disabled
            on:click=debounced_action
        >

            <Icon icon=icon />
        </button>
    }
}

#[component]
pub fn HistoryMove(
    node: Node<i32, TreeNode>,
    current_path: Memo<Vec<i32>>,
    has_children: bool,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisStore>();
    let game_state = expect_context::<GameStateStore>();
    if let (Ok(Some(value)), Ok(node_id)) = (node.get_value(), node.get_node_id()) {
        let is_current =
            Signal::derive(move || analysis.current_node().get().is_some_and(|c| c == node));
        let class = move || {
            let margin = if has_children { "" } else { "ml-[15px] " };
            let bg_color = if is_current() {
                "bg-orange-twilight "
            } else {
                ""
            };
            format!("{margin}w-fit transition-transform duration-300 transform hover:bg-pillbug-teal dark:hover:bg-pillbug-teal {bg_color}active:scale-95")
        };
        let onclick = move |_| {
            analysis.update_node(node_id);
            analysis.sync_game_state(game_state);
        };
        let history_index = value.turn - 1;
        let game_state = expect_context::<GameStateStore>();
        let repetitions = Signal::derive(move || game_state.state().get().repeating_moves.clone());
        let rep = move || {
            if repetitions.with(|r| r.contains(&history_index))
                && current_path.with(|p| p.contains(&node_id))
            {
                String::from(" ↺")
            } else {
                String::new()
            }
        };
        view! {
            <div class=class on:click=onclick>
                {move || format!("{}. {} {} {}", value.turn, value.piece, value.position, rep())}
            </div>
        }
        .into_any()
    } else {
        view! { <div>"Invalid node"</div> }.into_any()
    }
}

#[component]
pub fn CollapsibleMove(
    node: Node<i32, TreeNode>,
    current_path: Memo<Vec<i32>>,
    inner: AnyView,
) -> impl IntoView {
    let closed_toggles = expect_context::<ToggleStates>().0;
    if let Ok(node_id) = node.get_node_id() {
        let is_open = !closed_toggles.get_untracked().contains(&node_id);
        let (open, set_open) = signal(is_open);
        let onclick = move |_| {
            let s = !open();
            closed_toggles.update_untracked(|t| {
                if s {
                    t.remove(&node_id);
                } else {
                    t.insert(node_id);
                }
            });
            set_open(s);
        };
        view! {
            <div class="flex">
                <button on:click=onclick>
                    <svg
                        width="15"
                        height="15"
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points=move || {
                            if open() { "19 12 12 19 5 12" } else { "12 5 19 12 12 19" }
                        }></polyline>
                    </svg>
                </button>
                <HistoryMove current_path node=node.clone() has_children=true />
            </div>
            <div class=move || {
                format!("nested-content {}", if open() { "" } else { "hidden" })
            }>{inner}</div>
        }
        .into_any()
    } else {
        view! { <div>"Invalid node"</div> }.into_any()
    }
}
