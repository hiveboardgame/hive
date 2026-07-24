use crate::{
    components::organisms::{
        analysis::{atoms::HistoryRow, AnalysisHistoryControls, DownloadTree, LoadTree},
        reserve::{Alignment, Reserve},
    },
    hiveground::HivegroundInteraction,
    providers::analysis::AnalysisContext,
};
use hive_lib::{Board, Color};
use leptos::{html, leptos_dom::helpers::request_animation_frame, prelude::*};
use leptos_use::use_resize_observer;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(super) const ROW_HEIGHT: usize = 32;
const OVERSCAN_ROWS: usize = 8;
const ACTION_BUTTON_CLASS: &str =
    "ui-button ui-button-secondary ui-button-sm min-h-9 w-full whitespace-normal px-2 py-1 text-center text-xs leading-tight";

fn visible_row_bounds(row_count: usize, scroll_top: f64, viewport_height: f64) -> (usize, usize) {
    let first_visible =
        ((scroll_top / ROW_HEIGHT as f64).floor() as usize).min(row_count.saturating_sub(1));
    let first = first_visible.saturating_sub(OVERSCAN_ROWS).min(row_count);
    let visible_count = (viewport_height / ROW_HEIGHT as f64).ceil() as usize;
    let end = first
        .saturating_add(visible_count)
        .saturating_add(OVERSCAN_ROWS * 2)
        .min(row_count);
    (first, end)
}

#[component]
pub fn History(
    interaction: HivegroundInteraction,
    history_board: Memo<Board>,
    #[prop(optional)] mobile: bool,
    #[prop(optional)] hide_controls: bool,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>();
    let store = analysis.store;
    let reserve_class =
        "flex flex-col py-1 px-2 rounded border border-black/5 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted";
    let has_history = Memo::new(move |_| store.has_moves());
    let row_count = Memo::new(move |_| store.visible_row_count());
    let scroll_top = RwSignal::new(0_f64);
    let viewport_height = RwSignal::new(320_f64);
    let scroll_ref = NodeRef::<html::Div>::new();
    let mounted = Arc::new(AtomicBool::new(true));
    let cleanup_mounted = Arc::clone(&mounted);
    on_cleanup(move || cleanup_mounted.store(false, Ordering::Release));
    use_resize_observer(scroll_ref, move |entries, _observer| {
        let next_viewport_height = entries[0].content_rect().height();
        if viewport_height.get_untracked() != next_viewport_height {
            viewport_height.set(next_viewport_height);
        }
    });
    let visible_bounds = Memo::new(move |_| {
        visible_row_bounds(row_count.get(), scroll_top.get(), viewport_height.get())
    });
    Effect::new(move |_| {
        let selected = store.selected_node_id();
        let selected_index = store.visible_row_index(selected);
        let Some(selected_index) = selected_index else {
            return;
        };
        let mounted = Arc::clone(&mounted);
        request_animation_frame(move || {
            if !mounted.load(Ordering::Acquire) {
                return;
            }
            let Some(element) = scroll_ref.get_untracked() else {
                return;
            };
            let client_height = element.client_height();
            let row_top = selected_index.saturating_mul(ROW_HEIGHT) as i32;
            let row_bottom = row_top.saturating_add(ROW_HEIGHT as i32);
            let viewport_top = element.scroll_top();
            let viewport_bottom = viewport_top.saturating_add(client_height);
            let next_scroll_top = if row_top < viewport_top {
                row_top
            } else if row_bottom > viewport_bottom {
                row_bottom.saturating_sub(client_height)
            } else {
                viewport_top
            };
            if next_scroll_top != viewport_top {
                element.set_scroll_top(next_scroll_top);
            }
            let client_height = f64::from(client_height);
            if viewport_height.get_untracked() != client_height {
                viewport_height.set(client_height);
            }
            let next_scroll_top = f64::from(next_scroll_top);
            if scroll_top.get_untracked() != next_scroll_top {
                scroll_top.set(next_scroll_top);
            }
        });
    });
    let viewbox_str = "-32 -40 250 120";
    view! {
        <div class="flex flex-col gap-3 min-h-0 size-full">
            <Show when=move || !hide_controls>
                <AnalysisHistoryControls />
            </Show>
            <Show when=move || !mobile>
                <div class=reserve_class>
                    <Reserve
                        alignment=Alignment::DoubleRow
                        color=Color::Black
                        viewbox_str
                        interaction
                        history_board
                    />
                    <Reserve
                        alignment=Alignment::DoubleRow
                        color=Color::White
                        viewbox_str
                        interaction
                        history_board
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
                <button
                    on:click=move |_| store.promote_current_variation(true)
                    class=ACTION_BUTTON_CLASS
                >
                    "Make main line"
                </button>
                <button
                    on:click=move |_| store.promote_current_variation(false)
                    class=ACTION_BUTTON_CLASS
                >
                    "Promote variation"
                </button>
            </div>
            <div
                node_ref=scroll_ref
                class="overflow-y-auto p-2 min-h-0 text-sm rounded border grow border-black/5 bg-even-light/70 dark:border-white/10 dark:bg-surface-field"
                on:scroll=move |event| {
                    let element = event_target::<web_sys::HtmlElement>(&event);
                    let next_scroll_top = f64::from(element.scroll_top());
                    if scroll_top.get_untracked() != next_scroll_top {
                        scroll_top.set(next_scroll_top);
                    }
                }
            >
                <div
                    class="relative w-full"
                    style:height=move || {
                        format!("{}px", row_count.get().saturating_mul(ROW_HEIGHT))
                    }
                >
                    <div
                        class="absolute inset-x-0"
                        style:top=move || {
                            format!(
                                "{}px",
                                visible_bounds.with(|(first, _)| first.saturating_mul(ROW_HEIGHT)),
                            )
                        }
                    >
                        <For
                            each=move || {
                                let (first, end) = visible_bounds.get();
                                store.visible_rows_in(first..end)
                            }
                            key=|row| (row.node_id, row.indent, row.has_variations)
                            children=move |row| {
                                let value = store.node_value_untracked(row.node_id);
                                // Document replacement remounts History before node IDs can be reused.
                                view! { <HistoryRow row value /> }
                            }
                        />
                    </div>
                </div>
            </div>
            <Show when=move || !mobile>
                <AnalysisHistoryControls />
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_row_bounds_include_overscan_and_clamp_to_history() {
        for (row_count, scroll_top, viewport_height, expected) in [
            (0, 0.0, 320.0, (0, 0)),
            (100, 0.0, 64.0, (0, 18)),
            (100, 320.0, 64.0, (2, 20)),
            (3, 52_000.0, 320.0, (0, 3)),
        ] {
            assert_eq!(
                visible_row_bounds(row_count, scroll_top, viewport_height),
                expected,
            );
        }
    }
}
