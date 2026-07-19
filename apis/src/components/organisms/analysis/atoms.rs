use crate::{
    components::{
        atoms::history_nav_button::HistoryNavButton,
        molecules::annotation_toolbar::AnnotationToggle,
    },
    hooks::history_nav::{
        can_navigate_analysis_history,
        navigate_analysis_history,
        scroll_move_into_view,
        AnalysisHistoryNavigation as HistoryNavigation,
    },
    pages::analysis::ToggleStates,
    providers::{
        analysis::{AnalysisSignal, TreeNode},
        game_state::{GameStateStore, GameStateStoreFields},
    },
};
use leptos::prelude::*;
use leptos_icons::Icon;
use tree_ds::prelude::Node;

#[component]
pub fn AnalysisHistoryControls(
    #[prop(optional)] scroll_on_navigate: bool,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let class = if compact {
        "grid grid-cols-5 gap-1 px-1 pb-1 [&>*]:w-full"
    } else {
        "grid grid-cols-5 gap-2 [&>*]:w-full"
    };

    view! {
        <div class=class>
            <HistoryButton action=HistoryNavigation::First scroll_on_navigate=scroll_on_navigate />
            <HistoryButton
                action=HistoryNavigation::Previous
                scroll_on_navigate=scroll_on_navigate
            />
            <HistoryButton action=HistoryNavigation::Next scroll_on_navigate=scroll_on_navigate />
            <UndoButton />
            <AnnotationToggle class="ui-board-nav-button" active_tw_classes="ui-segmented-active" />
        </div>
    }
}

#[component]
pub fn UndoButton() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateStore>();
    let analysis = StoredValue::new(analysis);
    let is_disabled = move || {
        analysis
            .get_value()
            .tree
            .with(|a| a.current_node.is_none() || a.is_at_start())
    };
    let undo = move |_| {
        let analysis = analysis.get_value();
        analysis.tree.update(|a| {
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
        analysis.sync_reserve_from_game_state(game_state);
    };

    view! {
        <button class="ui-board-nav-button" on:click=undo prop:disabled=is_disabled>
            <Icon icon=icondata_bi::BiUndoRegular attr:class="size-6" />
        </button>
    }
}

#[component]
pub fn HistoryButton(
    action: HistoryNavigation,
    #[prop(optional)] scroll_on_navigate: bool,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateStore>();
    let cloned_action = action;
    let icon = match action {
        HistoryNavigation::First => icondata_ai::AiFastBackwardFilled,
        HistoryNavigation::Next => icondata_ai::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata_ai::AiStepBackwardFilled,
    };

    let is_disabled = move || {
        analysis
            .tree
            .with(|analysis| !can_navigate_analysis_history(analysis, cloned_action))
    };
    let hold_reserve_sync = analysis.hold_reserve_sync;
    let on_press = Callback::new(move |()| {
        if navigate_analysis_history(action, analysis.tree, game_state) {
            analysis.sync_reserve_later_from_game_state(game_state);
            if scroll_on_navigate {
                scroll_move_into_view();
            }
        }
    });

    view! {
        <HistoryNavButton disabled=is_disabled on_press=on_press on_pointerdown=hold_reserve_sync>
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
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateStore>();
    if let Ok(node_id) = node.get_node_id() {
        let is_current =
            Memo::new(move |_| current_path.with(|path| path.first() == Some(&node_id)));
        let class = move || {
            let margin = if has_children { "" } else { "ml-6 " };
            let state_class = if is_current.get() {
                "bg-orange-twilight text-gray-950 "
            } else {
                "text-gray-800 dark:text-gray-100 "
            };
            format!("{margin}w-fit cursor-pointer rounded px-2 py-1 font-mono text-xs transition-colors active:scale-95 hover:bg-blue-light/70 dark:hover:bg-pillbug-teal/15 {state_class}")
        };
        let onclick = move |_| {
            analysis.tree.update(|a| {
                a.update_node(node_id, Some(game_state));
            });
            analysis.sync_reserve_from_game_state(game_state);
        };
        let value = node.get_value().ok().flatten();
        let history_index = value.as_ref().map(|value| value.turn - 1);
        let state = game_state.state();
        let repetitions = Memo::new(move |_| state.with(|state| state.repeating_moves.clone()));
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
            <div class="flex gap-1 items-center">
                <button
                    class="w-6 h-6 ui-button ui-button-ghost ui-button-tiny shrink-0"
                    on:click=onclick
                >
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
