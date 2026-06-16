use crate::{
    components::atoms::history_nav_button::HistoryNavButton,
    hooks::history_nav::{
        can_navigate_analysis_history,
        navigate_analysis_history,
        scroll_move_into_view,
        AnalysisHistoryNavigation as HistoryNavigation,
    },
    pages::analysis::ToggleStates,
    providers::{
        analysis::{AnalysisSignal, TreeNode},
        game_state::GameStateSignal,
    },
};
use leptos::prelude::*;
use leptos_icons::Icon;
use tree_ds::prelude::Node;

#[component]
pub fn UndoButton() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateSignal>();
    let analysis = StoredValue::new(analysis);
    let is_disabled = move || {
        analysis
            .get_value()
            .0
            .with(|a| a.current_node.is_none() || a.is_at_start())
    };
    let undo = move |_| {
        analysis.get_value().0.update(|a| {
            if let Some(node) = &a.current_node {
                if let Ok(node_id) = node.get_node_id() {
                    if let Ok(Some(new_current)) = node.get_parent_id() {
                        a.update_node(new_current, Some(game_state));
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

#[component]
pub fn HistoryButton(
    action: HistoryNavigation,
    #[prop(optional)] scroll_on_navigate: bool,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let cloned_action = action;
    let icon = match action {
        HistoryNavigation::First => icondata_ai::AiFastBackwardFilled,
        HistoryNavigation::Next => icondata_ai::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata_ai::AiStepBackwardFilled,
    };

    let is_disabled =
        move || analysis.with(|analysis| !can_navigate_analysis_history(analysis, cloned_action));
    let on_press = Callback::new(move |()| {
        if navigate_analysis_history(action, analysis, game_state) && scroll_on_navigate {
            scroll_move_into_view();
        }
    });

    view! {
        <HistoryNavButton disabled=is_disabled on_press=on_press>
            <Icon icon=icon />
        </HistoryNavButton>
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
    if let Ok(node_id) = node.get_node_id() {
        let is_current =
            Memo::new(move |_| current_path.with(|path| path.first() == Some(&node_id)));
        let class = move || {
            let margin = if has_children { "" } else { "ml-[15px] " };
            let bg_color = if is_current.get() {
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
        let value = node.get_value().ok().flatten();
        let history_index = value.as_ref().map(|value| value.turn - 1);
        let repetitions =
            create_read_slice(game_state.signal, |gs| gs.state.repeating_moves.clone());
        let rep = move || {
            if history_index
                .is_some_and(|history_index| repetitions.with(|r| r.contains(&history_index)))
                && current_path.with(|p| p.contains(&node_id))
            {
                String::from(" ↺")
            } else {
                String::new()
            }
        };
        view! {
            <div
                class=class
                data-history-current=move || is_current.get().to_string()
                on:click=onclick
            >
                {move || {
                    value
                        .as_ref()
                        .map(|value| {
                            format!("{}. {} {} {}", value.turn, value.piece, value.position, rep())
                        })
                        .unwrap_or_else(|| String::from("0."))
                }}
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
