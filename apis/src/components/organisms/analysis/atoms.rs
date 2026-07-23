use crate::{
    components::{
        atoms::history_nav_button::HistoryNavButton,
        molecules::{annotation_toolbar::AnnotationToggle, modal::Modal},
    },
    hooks::history_nav::{
        can_navigate_analysis_history,
        navigate_analysis_history,
        AnalysisHistoryNavigation as HistoryNavigation,
    },
    providers::{
        analysis::{AnalysisContext, MoveDelta, VisibleRow},
        game_state::{GameStateStore, GameStateStoreFields},
    },
};
use leptos::{html::Dialog, prelude::*};
use leptos_icons::Icon;

#[component]
pub fn AnalysisHistoryControls(#[prop(optional)] compact: bool) -> impl IntoView {
    let class = if compact {
        "grid grid-cols-5 gap-1 px-1 pb-1 [&>*]:w-full"
    } else {
        "grid grid-cols-5 gap-2 [&>*]:w-full"
    };

    view! {
        <div class=class>
            <HistoryButton action=HistoryNavigation::First />
            <HistoryButton action=HistoryNavigation::Previous />
            <HistoryButton action=HistoryNavigation::Next />
            <DeleteBranchButton />
            <AnnotationToggle class="ui-board-nav-button" active_tw_classes="ui-segmented-active" />
        </div>
    }
}

#[component]
fn DeleteBranchButton() -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let dialog_ref = NodeRef::<Dialog>::new();
    let pending = RwSignal::new(None);
    let is_disabled = move || analysis.store.is_at_start();
    let request_delete = move |_| {
        let Some(summary) = analysis.store.selected_subtree_summary() else {
            return;
        };
        pending.set(Some(summary));
        if let Some(dialog) = dialog_ref.get_untracked() {
            let _ = dialog.show_modal();
        }
    };
    let confirm_delete = move |_| {
        let Some(node_id) =
            pending.with_untracked(|pending| pending.as_ref().map(|summary| summary.node_id))
        else {
            return;
        };
        if analysis.store.delete_subtree(node_id, game_state) {
            analysis.sync_reserve_from_game_state(game_state);
        }
        pending.set(None);
        if let Some(dialog) = dialog_ref.get_untracked() {
            dialog.close();
        }
    };
    let summary_text = move || {
        pending
            .with(|pending| {
                pending.as_ref().map(|summary| {
                    let value = &summary.move_delta;
                    let branch = format!("{}. {} {}", value.turn, value.piece, value.position);
                    let suffix = if summary.node_count == 1 { "" } else { "s" };
                    format!(
                        "Delete {branch} and {} node{suffix}? This cannot be undone.",
                        summary.node_count,
                    )
                })
            })
            .unwrap_or_default()
    };

    view! {
        <button
            class="ui-board-nav-button"
            on:click=request_delete
            prop:disabled=is_disabled
            aria-label="Delete branch"
            title="Delete branch"
        >
            <Icon icon=icondata_bi::BiTrashAltRegular attr:class="size-6" />
        </button>
        <div class="contents">
            <Modal dialog_el=dialog_ref aria_label="Delete analysis branch">
                <div class="px-5 pb-5 space-y-4 w-80 max-w-[calc(100vw-2rem)]">
                    <div>
                        <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">
                            "Delete this branch?"
                        </h2>
                        <p class="mt-2 text-sm text-gray-600 dark:text-gray-300">{summary_text}</p>
                    </div>
                    <div class="flex flex-wrap gap-2 justify-end">
                        <form method="dialog">
                            <button
                                type="submit"
                                class="ui-button ui-button-secondary ui-button-sm"
                            >
                                "Cancel"
                            </button>
                        </form>
                        <button
                            type="button"
                            on:click=confirm_delete
                            class="ui-button ui-button-danger ui-button-sm"
                        >
                            "Delete branch"
                        </button>
                    </div>
                </div>
            </Modal>
        </div>
    }
}

#[component]
fn HistoryButton(action: HistoryNavigation) -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let icon = match action {
        HistoryNavigation::First => icondata_ai::AiFastBackwardFilled,
        HistoryNavigation::Next => icondata_ai::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata_ai::AiStepBackwardFilled,
    };

    let is_disabled = move || !can_navigate_analysis_history(analysis.store, action);
    let hold_reserve_sync = analysis.hold_reserve_sync;
    let on_press = Callback::new(move |()| {
        if navigate_analysis_history(action, analysis.store, game_state) {
            analysis.sync_reserve_later_from_game_state(game_state);
        }
    });

    view! {
        <HistoryNavButton disabled=is_disabled on_press=on_press on_pointerdown=hold_reserve_sync>
            <Icon icon=icon />
        </HistoryNavButton>
    }
}

#[component]
pub(super) fn HistoryRow(row: VisibleRow, value: Option<MoveDelta>) -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let node_id = row.node_id;
    let is_current = Memo::new(move |_| analysis.store.selected_node_id() == node_id);
    let variations_open = Memo::new(move |_| analysis.store.variations_open(node_id));
    let onclick = move |_| {
        analysis.store.select_node(node_id, game_state);
        analysis.sync_reserve_from_game_state(game_state);
    };
    let history_index = value.as_ref().and_then(|value| value.turn.checked_sub(1));
    let state = game_state.state();
    let rep = move || {
        if history_index.is_some_and(|history_index| {
            state.with(|state| state.repeating_moves.contains(&history_index))
        }) && analysis.store.node_is_on_current_path(node_id)
        {
            " ↺"
        } else {
            ""
        }
    };
    let label = move || {
        value
            .as_ref()
            .map(|value| {
                format!(
                    "{}. {} {}{}",
                    value.turn,
                    value.piece,
                    value.position,
                    rep()
                )
            })
            .unwrap_or_else(|| String::from("0."))
    };
    let height = format!("{}px", super::history::ROW_HEIGHT);
    let padding = format!("{}px", row.indent.saturating_mul(16));
    view! {
        <div class="flex items-center min-w-0" style:height=height style:padding-left=padding>
            <Show
                when=move || row.has_variations
                fallback=|| view! { <span class="w-6 shrink-0"></span> }
            >
                <button
                    class="w-6 h-6 ui-button ui-button-ghost ui-button-tiny shrink-0"
                    aria-expanded=move || variations_open.get().to_string()
                    on:click=move |event| {
                        event.stop_propagation();
                        analysis.store.toggle_variations(node_id);
                    }
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
                            if variations_open.get() {
                                "19 12 12 19 5 12"
                            } else {
                                "12 5 19 12 12 19"
                            }
                        }></polyline>
                    </svg>
                </button>
            </Show>
            <div
                class="py-1 px-2 min-w-0 font-mono text-xs rounded transition-colors cursor-pointer active:scale-95 truncate dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70"
                class=("bg-orange-twilight", move || is_current.get())
                class=("text-gray-950", move || is_current.get())
                class=("text-gray-800", move || !is_current.get())
                class=("dark:text-gray-100", move || !is_current.get())
                data-history-node-id=node_id.get().to_string()
                data-history-current=move || is_current.get().to_string()
                on:click=onclick
            >
                {label}
            </div>
        </div>
    }
}
