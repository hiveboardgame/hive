use crate::components::molecules::game_row::GameRow;
use crate::i18n::*;
use crate::responses::GameResponse;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::hooks::use_query_map;
use shared_types::{FinishedGamesQueryOptions, ALLOWED_BATCH_SIZES};

const PAGE_SIZE_SELECT_CLASS: &str =
    "select select-bordered select-sm w-16 rounded-lg border-2 border-gray-300 dark:border-gray-600 bg-white text-gray-900 dark:bg-gray-800 dark:text-gray-100 h-11 min-h-11 font-medium focus:ring-2 focus:ring-pillbug-teal/50";

const PAGINATION_BTN_CLASS: &str =
    "inline-flex items-center justify-center size-11 rounded-lg border-2 border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-gray-100 hover:border-pillbug-teal/50 dark:hover:bg-gray-700 dark:hover:border-pillbug-teal/50 shadow-sm";

#[component]
pub fn ArchiveGameList(
    games: RwSignal<Vec<GameResponse>>,
    is_loading: Signal<bool>,
    has_searched: Signal<bool>,
    total: RwSignal<Option<i64>>,
    page: Signal<usize>,
    batch_size: Signal<usize>,
    on_page_change: Callback<usize>,
    #[prop(optional)] draft_options: Option<RwSignal<FinishedGamesQueryOptions>>,
    #[prop(optional)] on_search: Option<Callback<()>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let queries = use_query_map();

    let total_pages = Signal::derive(move || {
        let size = batch_size.get();
        total
            .get()
            .map(|t| {
                if t == 0 {
                    0
                } else {
                    ((t as usize) + size - 1) / size
                }
            })
            .unwrap_or(0)
    });
    let has_prev = Signal::derive(move || page.get() > 1);
    let has_next = Signal::derive(move || page.get() < total_pages.get());
    let can_first = has_prev;
    let can_last = has_next;

    view! {
        <div class="space-y-4">
            <Show when=move || total.get().is_some()>
                <div class="max-w-5xl mx-auto px-4 py-3 rounded-xl border border-gray-200 dark:border-gray-700 bg-white/60 dark:bg-gray-900/60 shadow-sm">
                    <div class="flex flex-wrap items-center justify-between gap-4">
                        <p class="text-sm font-medium text-gray-700 dark:text-gray-300 shrink-0">
                        {move || {
                            let t = total.get().unwrap_or(0);
                            let p = page.get();
                            let size = batch_size.get();
                            let total_pg = total_pages.get();
                            if total_pg > 0 {
                                let start = (p - 1) * size + 1;
                                let end = (p * size).min(t as usize);
                                t_string!(i18n, archive.page_info, page = p, total_pages = total_pg, start = start, end = end, total = t)
                            } else {
                                t_string!(i18n, archive.games_loaded_count, loaded = 0, total = t)
                            }
                        }}
                        </p>
                        <div class="flex flex-wrap items-center gap-3">
                            <Show when=move || draft_options.is_some() && on_search.is_some()>
                                <div class="flex items-center gap-2 shrink-0">
                                    <label class="text-sm font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap">
                                        {t!(i18n, archive.page_size)}
                                    </label>
                                    <select
                                        class=PAGE_SIZE_SELECT_CLASS
                                    prop:value=Signal::derive(move || batch_size.get().to_string())
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        if let (Some(ref opts), Some(ref search)) = (draft_options.as_ref(), on_search.as_ref()) {
                                            if let Ok(size) = val.parse::<usize>() {
                                                if ALLOWED_BATCH_SIZES.contains(&size) {
                                                    opts.update(|o| {
                                                        o.batch_size = size;
                                                        o.page = 1;
                                                    });
                                                    search.run(());
                                                }
                                            }
                                        }
                                    }
                                >
                                    {ALLOWED_BATCH_SIZES
                                        .iter()
                                        .map(|&size| {
                                            view! {
                                                <option value=size.to_string()>
                                                    {size.to_string()}
                                                </option>
                                            }
                                        })
                                        .collect_view()}
                                    </select>
                                </div>
                            </Show>
                            <div class="flex items-center gap-1.5">
                        <button
                            type="button"
                            class=PAGINATION_BTN_CLASS
                            disabled=move || !can_first.get()
                            aria-label=move || t_string!(i18n, archive.first_page).to_string()
                            on:click=move |_| {
                                if can_first.get_untracked() {
                                    on_page_change.run(1);
                                }
                            }
                        >
                            <Icon icon=icondata_ai::AiFastBackwardFilled attr:class="size-5" />
                        </button>
                        <button
                            type="button"
                            class=PAGINATION_BTN_CLASS
                            disabled=move || !has_prev.get()
                            aria-label=move || t_string!(i18n, archive.prev_page).to_string()
                            on:click=move |_| {
                                if has_prev.get_untracked() {
                                    on_page_change.run(page.get_untracked() - 1);
                                }
                            }
                        >
                            <Icon icon=icondata_ai::AiStepBackwardFilled attr:class="size-5" />
                        </button>
                        <button
                            type="button"
                            class=PAGINATION_BTN_CLASS
                            disabled=move || !has_next.get()
                            aria-label=move || t_string!(i18n, archive.next_page).to_string()
                            on:click=move |_| {
                                if has_next.get_untracked() {
                                    on_page_change.run(page.get_untracked() + 1);
                                }
                            }
                        >
                            <Icon icon=icondata_ai::AiStepForwardFilled attr:class="size-5" />
                        </button>
                        <button
                            type="button"
                            class=PAGINATION_BTN_CLASS
                            disabled=move || !can_last.get()
                            aria-label=move || t_string!(i18n, archive.last_page).to_string()
                            on:click=move |_| {
                                if can_last.get_untracked() {
                                    on_page_change.run(total_pages.get_untracked());
                                }
                            }
                        >
                            <Icon icon=icondata_ai::AiFastForwardFilled attr:class="size-5" />
                        </button>
                            </div>
                            <a
                                href=move || format!("/archive{}", queries.get().to_query_string())
                                class="text-sm link link-hover shrink-0 font-medium"
                            >
                                {t!(i18n, archive.permalink)}
                            </a>
                        </div>
                    </div>
                </div>
            </Show>
            <div class="space-y-4 max-w-5xl mx-auto w-full">
                <Show when=move || has_searched.get()>
                    <div class="flex flex-col">
                        <div class="min-h-0 rounded-xl sm:grid sm:grid-cols-2 sm:content-start lg:grid-cols-3 gap-3 sm:gap-4">
                            <For
                                each=move || games.get()
                                key=|game| game.game_id.clone()
                                let:game
                            >
                                <GameRow game />
                            </For>
                        </div>
                        <Show when=move || games.with(|g| g.is_empty()) && !is_loading.get()>
                            <p class="mt-6 py-8 text-center text-gray-600 dark:text-gray-400 rounded-xl bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-700">
                                {t!(i18n, archive.no_games_found)}
                            </p>
                        </Show>
                        <Show when=is_loading>
                            <p class="mt-6 py-8 text-center text-gray-600 dark:text-gray-400 rounded-xl bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-700 animate-pulse">
                                {t!(i18n, archive.loading_games)}
                            </p>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    }
}
