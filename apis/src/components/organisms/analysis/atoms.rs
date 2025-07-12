use crate::pages::analysis::ToggleStates;
use crate::providers::analysis::{AnalysisSignal, TreeNode};
use crate::providers::game_state::GameStateSignal;
use leptos::{html, prelude::*};
use leptos_icons::Icon;
use tree_ds::prelude::Node;

#[component]
pub fn UndoButton() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateSignal>();
    let analysis = StoredValue::new(analysis.clone());
    let is_disabled = move || analysis.get_value().0.with(|a| a.current_node.is_none());
    let undo = move |_| {
        analysis.get_value().0.update(|a| {
            if let Some(node) = &a.current_node {
                let node_id = node.get_node_id();
                let new_current = node.get_parent_id();
                if let Some(new_current) = new_current {
                    a.update_node(new_current, Some(game_state));
                    if let Ok(tree) = a.tree.get_subtree(&node_id, None) {
                        tree.get_nodes().iter().for_each(|n| {
                            a.hashes.remove_by_right(&n.get_node_id());
                        });
                    };
                    a.tree
                        .remove_node(
                            &node_id,
                            tree_ds::prelude::NodeRemovalStrategy::RemoveNodeAndChildren,
                        )
                        .unwrap();
                } else {
                    a.reset();
                }
            }
        });
    };

    view! {
        <button
            class="flex justify-center place-items-center m-1 h-7 rounded-md border-2 border-cyan-500 drop-shadow-lg transition-transform duration-300 transform hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 dark:border-button-twilight disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
            on:click=undo
            prop:disabled=is_disabled
        >
            <Icon icon=icondata::BiUndoRegular attr:class="w-6 h-6" />
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
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let cloned_action = action.clone();
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 m-1 h-7 rounded-md border-cyan-500 dark:border-button-twilight border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let icon = match action {
        HistoryNavigation::First => icondata::AiFastBackwardFilled,
        HistoryNavigation::Next => icondata::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata::AiStepBackwardFilled,
    };

    let is_disabled = move || {
        analysis.with(|analysis| {
            if let Some(n) = &analysis.current_node {
                match cloned_action {
                    HistoryNavigation::First => analysis
                        .tree
                        .get_ancestor_ids(&n.get_node_id())
                        .map_or(true, |ids| ids.is_empty()),
                    HistoryNavigation::Next => n.get_children_ids().is_empty(),
                    HistoryNavigation::Previous => n.get_parent_id().is_none(),
                }
            } else {
                false
            }
        })
    };
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        let updated_node_id = analysis.with(|a| {
            a.current_node.as_ref().and_then(|n| match action {
                HistoryNavigation::First => a
                    .tree
                    .get_ancestor_ids(&n.get_node_id())
                    .map_or(None, |ids| ids.last().cloned()),
                HistoryNavigation::Next => n.get_children_ids().first().cloned(),
                HistoryNavigation::Previous => n.get_parent_id(),
            })
        });
        if let Some(updated_node_id) = updated_node_id {
            analysis.update(|a| {
                a.update_node(updated_node_id, Some(game_state));
            });
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
            prop:disabled=is_disabled
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
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let value = node.get_value().unwrap();
    let node_id = node.get_node_id();
    let class = move || {
        let margin = if has_children { "" } else { "ml-[15px] " };
        let bg_color = if current_path.with(|path| path.first() == Some(&node_id)) {
            "bg-orange-twilight "
        } else {
            ""
        };
        format!("{margin}w-fit transition-transform duration-300 transform hover:bg-pillbug-teal dark:hover:bg-pillbug-teal {bg_color}active:scale-95")
    };
    let onclick = move |_| {
        analysis.update(|a| {
            a.update_node(node_id, Some(game_state));
        });
    };
    let history_index = value.turn - 1;
    let game_state = expect_context::<GameStateSignal>();
    let repetitions = create_read_slice(game_state.signal, |gs| gs.state.repeating_moves.clone());
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
}

#[component]
pub fn CollapsibleMove(
    node: Node<i32, TreeNode>,
    current_path: Memo<Vec<i32>>,
    inner: AnyView,
) -> impl IntoView {
    let closed_toggles = expect_context::<ToggleStates>().0;
    let node_id = node.get_node_id();
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
}
