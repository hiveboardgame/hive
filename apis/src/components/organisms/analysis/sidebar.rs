use crate::{
    components::organisms::{
        analysis::{AnalysisHistoryControls, Evals, History, OpeningExplorer},
        reserve::{Alignment, Reserve},
    },
    hiveground::HivegroundInteraction,
    providers::game_eval::evals_visible,
};
use hive_lib::{Board, Color};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[derive(Clone, Copy, PartialEq)]
enum AnalysisTab {
    History,
    Explorer,
    Evals,
}

impl AnalysisTab {
    fn from_query(value: &str) -> Option<Self> {
        match value {
            "history" => Some(AnalysisTab::History),
            "explorer" => Some(AnalysisTab::Explorer),
            "evals" => Some(AnalysisTab::Evals),
            _ => None,
        }
    }
}

/// `?tab=evals` opens analysis on that tab, so links can point straight at a game's eval.
fn requested_tab() -> AnalysisTab {
    use_query_map()
        .with_untracked(|query| {
            query
                .get("tab")
                .as_deref()
                .and_then(AnalysisTab::from_query)
        })
        .unwrap_or(AnalysisTab::History)
}

/// The tab on screen: a shared `?tab=evals` link opens History for viewers who cannot see evals.
fn shown_tab(tab: RwSignal<AnalysisTab>) -> Signal<AnalysisTab> {
    let visible = evals_visible();
    Signal::derive(move || match tab() {
        AnalysisTab::Evals if !visible.get() => AnalysisTab::History,
        tab => tab,
    })
}

#[component]
fn AnalysisTabList(tab: RwSignal<AnalysisTab>, shown: Signal<AnalysisTab>) -> impl IntoView {
    let trigger_class = move |name: AnalysisTab| {
        move || {
            format!(
                "ui-board-tab-trigger cursor-pointer {}",
                if shown() == name {
                    "ui-segmented-active hover:bg-button-dawn dark:hover:bg-button-twilight"
                } else {
                    "hover:bg-blue-light/70 dark:hover:bg-pillbug-teal/15"
                },
            )
        }
    };

    let shown_evals = evals_visible();
    view! {
        <div class="sticky top-0 z-10 ui-board-tab-list">
            <button
                type="button"
                class=trigger_class(AnalysisTab::History)
                aria-pressed=move || (shown() == AnalysisTab::History).to_string()
                on:click=move |_| tab.set(AnalysisTab::History)
            >
                "History"
            </button>
            <button
                type="button"
                class=trigger_class(AnalysisTab::Explorer)
                aria-pressed=move || (shown() == AnalysisTab::Explorer).to_string()
                on:click=move |_| tab.set(AnalysisTab::Explorer)
            >
                "Explorer"
            </button>
            <Show when=move || shown_evals.get()>
                <button
                    type="button"
                    class=trigger_class(AnalysisTab::Evals)
                    aria-pressed=move || (shown() == AnalysisTab::Evals).to_string()
                    on:click=move |_| tab.set(AnalysisTab::Evals)
                >
                    "Evals"
                </button>
            </Show>
        </div>
    }
}

#[component]
pub fn AnalysisSidebar(
    interaction: HivegroundInteraction,
    history_board: Memo<Board>,
) -> impl IntoView {
    let tab = RwSignal::new(requested_tab());
    let shown = shown_tab(tab);
    let reserve_class =
        "flex flex-col py-1 px-2 rounded border border-black/5 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted";
    view! {
        <div class="flex flex-col flex-1 min-h-0 select-none ui-board-side-panel">
            <AnalysisTabList tab shown />
            <div class="flex overflow-y-auto flex-col flex-grow p-3 min-h-0">
                <Show when=move || shown() == AnalysisTab::History>
                    <History interaction history_board />
                </Show>
                <Show when=move || shown() == AnalysisTab::Explorer>
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
                        <OpeningExplorer />
                    </div>
                </Show>
                <Show when=move || shown() == AnalysisTab::Evals>
                    <Evals />
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
) -> impl IntoView {
    let tab = RwSignal::new(requested_tab());
    let shown = shown_tab(tab);

    view! {
        <div class="flex flex-col min-h-0 select-none h-[calc(100svh-2.5rem)] shrink-0 ui-board-side-panel">
            <AnalysisTabList tab shown />
            <div class="flex overflow-y-auto flex-col flex-grow p-3 min-h-0">
                <Show when=move || shown() == AnalysisTab::History>
                    <History mobile=true interaction history_board />
                </Show>
                <Show when=move || shown() == AnalysisTab::Explorer>
                    <OpeningExplorer />
                </Show>
                <Show when=move || shown() == AnalysisTab::Evals>
                    <Evals />
                </Show>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::AnalysisTab;

    #[test]
    fn tab_names_in_the_query_pick_the_tab() {
        assert!(AnalysisTab::from_query("evals") == Some(AnalysisTab::Evals));
        assert!(AnalysisTab::from_query("explorer") == Some(AnalysisTab::Explorer));
        assert!(AnalysisTab::from_query("history") == Some(AnalysisTab::History));
        assert!(AnalysisTab::from_query("Evals").is_none());
    }
}
