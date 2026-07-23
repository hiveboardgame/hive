use crate::{
    components::organisms::{
        analysis::{AnalysisHistoryControls, AnalysisPreviewSnapshot, History, OpeningExplorer},
        reserve::{Alignment, Reserve},
    },
    hiveground::HivegroundInteraction,
};
use hive_lib::{Board, Color};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum AnalysisTab {
    History,
    Explorer,
}

#[component]
fn AnalysisTabList(tab: RwSignal<AnalysisTab>) -> impl IntoView {
    let trigger_class = move |name: AnalysisTab| {
        move || {
            format!(
                "ui-board-tab-trigger cursor-pointer {}",
                if tab() == name {
                    "ui-segmented-active hover:bg-button-dawn dark:hover:bg-button-twilight"
                } else {
                    "hover:bg-blue-light/70 dark:hover:bg-pillbug-teal/15"
                },
            )
        }
    };

    view! {
        <div class="sticky top-0 z-10 ui-board-tab-list">
            <button
                type="button"
                class=trigger_class(AnalysisTab::History)
                aria-pressed=move || (tab() == AnalysisTab::History).to_string()
                on:click=move |_| tab.set(AnalysisTab::History)
            >
                "History"
            </button>
            <button
                type="button"
                class=trigger_class(AnalysisTab::Explorer)
                aria-pressed=move || (tab() == AnalysisTab::Explorer).to_string()
                on:click=move |_| tab.set(AnalysisTab::Explorer)
            >
                "Explorer"
            </button>
        </div>
    }
}

#[component]
pub fn AnalysisSidebar(
    interaction: HivegroundInteraction,
    history_board: Memo<Board>,
    preview_snapshot: RwSignal<Option<AnalysisPreviewSnapshot>>,
) -> impl IntoView {
    let tab = RwSignal::new(AnalysisTab::History);
    let reserve_class =
        "flex flex-col py-1 px-2 rounded border border-black/5 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted";
    view! {
        <div class="flex flex-col flex-1 min-h-0 select-none ui-board-side-panel">
            <AnalysisTabList tab />
            <div class="overflow-y-auto flex-grow p-3 min-h-0">
                <Show when=move || tab() == AnalysisTab::History>
                    <History interaction history_board />
                </Show>
                <Show when=move || tab() == AnalysisTab::Explorer>
                    <div class="flex flex-col gap-3 min-h-0">
                        <AnalysisHistoryControls />
                        <div class=reserve_class>
                            <Reserve
                                alignment=Alignment::DoubleRow
                                color=Color::Black
                                viewbox_str="-32 -40 250 120"
                                interaction
                                history_board
                            />
                            <Reserve
                                alignment=Alignment::DoubleRow
                                color=Color::White
                                viewbox_str="-32 -40 250 120"
                                interaction
                                history_board
                            />
                        </div>
                        <OpeningExplorer preview_snapshot />
                    </div>
                </Show>
            </div>
        </div>
    }
}

#[component]
pub fn AnalysisMobileHistoryControls() -> impl IntoView {
    view! { <AnalysisHistoryControls compact=true /> }
}

#[component]
pub fn AnalysisMobileTabs(
    interaction: HivegroundInteraction,
    history_board: Memo<Board>,
    preview_snapshot: RwSignal<Option<AnalysisPreviewSnapshot>>,
) -> impl IntoView {
    let tab = RwSignal::new(AnalysisTab::History);

    view! {
        <div class="flex flex-col min-h-0 select-none h-[calc(100svh-2.5rem)] shrink-0 ui-board-side-panel">
            <AnalysisTabList tab />
            <div class="flex overflow-y-auto flex-col flex-grow p-3 min-h-0">
                <Show when=move || tab() == AnalysisTab::History>
                    <History mobile=true hide_controls=true interaction history_board />
                </Show>
                <Show when=move || tab() == AnalysisTab::Explorer>
                    <OpeningExplorer preview_snapshot />
                </Show>
            </div>
        </div>
    }
}
