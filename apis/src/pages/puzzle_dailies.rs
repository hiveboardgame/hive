use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{
            empty_state::EmptyState,
            puzzle_nav::PuzzleNav,
            thumbnail_pieces::ThumbnailPieces,
        },
        organisms::puzzle_trainer::moves_label,
    },
    functions::puzzles::daily_archive,
    responses::DailyArchiveRow,
};
use leptos::prelude::*;
use shared_types::{start_state, OLD_DAILIES_COLLECTION};

#[component]
pub fn PuzzleDailies() -> impl IntoView {
    let page = RwSignal::new(0i64);
    let rows = Resource::new(move || page.get(), daily_archive);
    view! {
        <PageShell>
            <PageHeader title="Past puzzles of the day" />
            <PuzzleNav />
            <div class="flex flex-wrap gap-3 items-center text-sm">
                <a
                    href=format!("/puzzles?collection={OLD_DAILIES_COLLECTION}")
                    class="text-white whitespace-nowrap shadow min-h-11 ui-button ui-button-sm no-link-style bg-pillbug-teal hover:brightness-110"
                >
                    "Play old dailies →"
                </a>
                <span class="text-gray-600 dark:text-gray-400">
                    "Ones you have not tried come first; after that they come round again."
                </span>
            </div>
            <Transition fallback=|| {
                view! { <p class="text-sm">"Loading…"</p> }
            }>
                {move || Suspend::new(async move {
                    match rows.await {
                        Ok(rows) if rows.is_empty() => {
                            view! {
                                <EmptyState
                                    title="No puzzles of the day yet"
                                    message="Come back tomorrow."
                                />
                            }
                                .into_any()
                        }
                        Ok(rows) => {
                            let full_page = rows.len() == 30;
                            view! {
                                <ul class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                                    {rows
                                        .into_iter()
                                        .map(|row| view! { <DailyRow row /> })
                                        .collect_view()}
                                </ul>
                                <div class="flex gap-2 justify-center">
                                    <Show when=move || { page.get() > 0 }>
                                        <button
                                            type="button"
                                            class="min-h-11 ui-button ui-button-secondary"
                                            on:click=move |_| page.update(|page| *page -= 1)
                                        >
                                            "← Newer"
                                        </button>
                                    </Show>
                                    <Show when=move || full_page>
                                        <button
                                            type="button"
                                            class="min-h-11 ui-button ui-button-secondary"
                                            on:click=move |_| page.update(|page| *page += 1)
                                        >
                                            "Older →"
                                        </button>
                                    </Show>
                                </div>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! {
                                <p class="text-sm">"The puzzles of the day could not be loaded."</p>
                            }
                                .into_any()
                        }
                    }
                })}
            </Transition>
        </PageShell>
    }
}

#[component]
fn DailyRow(row: DailyArchiveRow) -> impl IntoView {
    let start = start_state(
        &row.game_type,
        row.history.as_deref(),
        row.position.as_deref(),
    )
    .ok();
    let side = match start.as_ref().map(|start| start.turn_color) {
        Some(hive_lib::Color::Black) => "Black",
        _ => "White",
    };
    let thumbnail = start.map(|start| {
        let board = StoredValue::new(start.board);
        view! {
            <div class="overflow-hidden w-20 h-20 rounded shrink-0 bg-black/5 dark:bg-white/5">
                <ThumbnailPieces board />
            </div>
        }
    });
    let (result, result_class) = match row.my_result {
        Some(true) => ("✓ Solved", "text-grasshopper-green"),
        Some(false) => ("Missed", "text-red-700 dark:text-red-400"),
        None => ("Not tried yet", "text-gray-600 dark:text-gray-400"),
    };
    view! {
        <li>
            <a
                href=format!("/puzzles/{}", row.nanoid)
                class="flex gap-3 items-center p-3 h-full rounded-lg border shadow-sm transition-colors no-link-style border-black/10 bg-even-light dark:border-white/10 dark:bg-surface-panel hover:border-pillbug-teal"
            >
                {thumbnail}
                <span class="flex flex-col min-w-0">
                    <span class="text-xs font-semibold tracking-wide uppercase text-pillbug-teal">
                        {row.day.format("%A %-d %B %Y").to_string()}
                    </span>
                    <span class="font-semibold">
                        {format!("{side} to win in {}", moves_label(row.solver_moves))}
                    </span>
                    <span class="text-xs text-gray-600 dark:text-gray-400">
                        {format!("Difficulty {}", row.rating)}
                    </span>
                    <span class=format!("text-sm font-semibold {result_class}")>{result}</span>
                </span>
            </a>
        </li>
    }
}
