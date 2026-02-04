use crate::components::molecules::game_row::GameRow;
use crate::i18n::*;
use crate::responses::GameResponse;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[component]
pub fn ArchiveGameList(
    games: RwSignal<Vec<GameResponse>>,
    is_loading: Signal<bool>,
    has_searched: Signal<bool>,
    total: RwSignal<Option<i64>>,
    page: Signal<usize>,
    batch_size: usize,
    on_page_change: Callback<usize>,
) -> impl IntoView {
    let i18n = use_i18n();
    let queries = use_query_map();

    let total_pages = Signal::derive(move || {
        total
            .get()
            .map(|t| {
                if t == 0 {
                    0
                } else {
                    ((t as usize) + batch_size - 1) / batch_size
                }
            })
            .unwrap_or(0)
    });
    let has_prev = Signal::derive(move || page.get() > 1);
    let has_next = Signal::derive(move || page.get() < total_pages.get());

    view! {
        <div class="space-y-2">
            <Show when=move || total.get().is_some()>
                <div class="max-w-5xl mx-auto px-4 flex flex-wrap items-center gap-2">
                    <p class="text-sm text-gray-700 dark:text-gray-300">
                        {move || {
                            let t = total.get().unwrap_or(0);
                            let p = page.get();
                            let total_pg = total_pages.get();
                            if total_pg > 0 {
                                let start = (p - 1) * batch_size + 1;
                                let end = (p * batch_size).min(t as usize);
                                t_string!(i18n, archive.page_info, page = p, total_pages = total_pg, start = start, end = end, total = t)
                            } else {
                                t_string!(i18n, archive.games_loaded_count, loaded = 0, total = t)
                            }
                        }}
                    </p>
                    <div class="flex gap-2">
                        <button
                            type="button"
                            class="btn btn-sm btn-ghost"
                            disabled=move || !has_prev.get()
                            on:click=move |_| {
                                if has_prev.get_untracked() {
                                    on_page_change.run(page.get_untracked() - 1);
                                }
                            }
                        >
                            {t!(i18n, archive.prev_page)}
                        </button>
                        <button
                            type="button"
                            class="btn btn-sm btn-ghost"
                            disabled=move || !has_next.get()
                            on:click=move |_| {
                                if has_next.get_untracked() {
                                    on_page_change.run(page.get_untracked() + 1);
                                }
                            }
                        >
                            {t!(i18n, archive.next_page)}
                        </button>
                    </div>
                    <a
                        href=move || format!("/archive{}", queries.get().to_query_string())
                        class="text-sm link link-hover"
                    >
                        {t!(i18n, archive.permalink)}
                    </a>
                </div>
            </Show>
            <div class="space-y-2">
                <Show when=move || has_searched.get()>
                    <div class="flex flex-col">
                        <div class="min-h-0 rounded-lg sm:grid sm:grid-cols-2 sm:content-start lg:grid-cols-3">
                            <For
                                each=move || games.get()
                                key=|game| game.game_id.clone()
                                let:game
                            >
                                <GameRow game />
                            </For>
                        </div>
                        <Show when=move || games.with(|g| g.is_empty()) && !is_loading.get()>
                            <p class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                {t!(i18n, archive.no_games_found)}
                            </p>
                        </Show>
                        <Show when=is_loading>
                            <p class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                {t!(i18n, archive.loading_games)}
                            </p>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    }
}
