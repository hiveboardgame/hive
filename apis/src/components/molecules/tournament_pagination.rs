use crate::{i18n::*, responses::TOURNAMENT_BROWSE_PAGE_SIZE};
use leptos::prelude::*;

#[component]
pub fn TournamentPagination(
    page: usize,
    total: usize,
    on_page_change: Callback<usize>,
) -> impl IntoView {
    let i18n = use_i18n();
    let pages = total.div_ceil(TOURNAMENT_BROWSE_PAGE_SIZE).max(1);
    let start = ((page - 1) * TOURNAMENT_BROWSE_PAGE_SIZE + 1).min(total);
    let end = (page * TOURNAMENT_BROWSE_PAGE_SIZE).min(total);
    // Keep comparisons outside attributes: leptosfmt strips braces needed around `>=`.
    let previous_disabled = page <= 1;
    let next_disabled = page >= pages;
    view! {
        <nav
            class="flex flex-wrap gap-y-2 gap-x-4 justify-between items-center text-sm"
            aria-label=move || t_string!(i18n, tournaments.list.tournament_pagination).to_string()
        >
            <p class="text-gray-600 dark:text-gray-300">
                {move || {
                    t_string!(
                        i18n, tournaments.list.showing_start_end_of_total_tournaments, start = (start), end = (end), total = (total)
                    )
                        .to_string()
                }}
            </p>
            <div class="flex gap-2 items-center">
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    disabled=previous_disabled
                    on:click=move |_| {
                        if page > 1 {
                            on_page_change.run(page - 1);
                        }
                    }
                >
                    {t!(i18n, common.previous)}
                </button>
                <span class="text-xs tabular-nums text-gray-600 dark:text-gray-300">
                    {move || {
                        t_string!(
                            i18n, tournaments.list.page_page_of_pages, page = (page), pages = (pages)
                        )
                            .to_string()
                    }}
                </span>
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    disabled=next_disabled
                    on:click=move |_| {
                        if page < pages {
                            on_page_change.run(page + 1);
                        }
                    }
                >
                    {t!(i18n, common.next)}
                </button>
            </div>
        </nav>
    }
}
