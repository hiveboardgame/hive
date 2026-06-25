use crate::{
    components::molecules::{empty_state::EmptyState, game_row::GameRow},
    hooks::clipboard_copy::use_clipboard_copy,
    i18n::*,
    responses::GameResponse,
};
use leptos::{html, prelude::*};
use leptos_icons::Icon;
use leptos_router::hooks::use_query_map;
use leptos_use::{
    use_intersection_observer_with_options,
    use_window,
    UseIntersectionObserverOptions,
};
use shared_types::{GamesQueryOptions, ALLOWED_BATCH_SIZES};

const PAGINATION_BTN_CLASS: &str = "inline-flex size-10 xs:size-11 max-[285px]:size-9 items-center justify-center rounded-lg shadow-sm transition-colors duration-200 active:scale-95 disabled:cursor-not-allowed disabled:opacity-40 ui-button-secondary";
const COMPACT_PAGINATION_BTN_CLASS: &str = "inline-flex size-10 items-center justify-center rounded-lg shadow-sm transition-colors duration-200 active:scale-95 disabled:cursor-not-allowed disabled:opacity-40 ui-button-secondary";

#[component]
pub fn ArchiveGameList(
    games: RwSignal<Vec<GameResponse>>,
    is_loading: Signal<bool>,
    has_searched: Signal<bool>,
    total: RwSignal<Option<i64>>,
    page: Signal<usize>,
    batch_size: Signal<usize>,
    on_page_change: Callback<usize>,
    #[prop(optional)] draft_options: Option<RwSignal<GamesQueryOptions>>,
    #[prop(optional)] on_search: Option<Callback<()>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let queries = use_query_map();

    // Full, shareable archive URL (with scheme) for the copy button below.
    let permalink_url = move || {
        let origin = use_window()
            .as_ref()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default();
        format!("{origin}/archive{}", queries.get().to_query_string())
    };
    let clipboard = use_clipboard_copy();
    let copy_state = clipboard.copied;
    let copy_text = clipboard.copy_text;
    let copy_permalink = move |_| copy_text(permalink_url());
    let copy_button_class = move || {
        if copy_state.get() {
            "ui-button ui-button-success ui-button-sm h-10 shrink-0 text-xs xs:h-11 xs:text-sm max-[285px]:px-3"
        } else {
            "ui-button ui-button-secondary ui-button-sm h-10 shrink-0 text-xs xs:h-11 xs:text-sm max-[285px]:px-3"
        }
    };

    let total_pages = Signal::derive(move || {
        let size = batch_size.get();
        total
            .get()
            .map(|t| {
                if t == 0 {
                    0
                } else {
                    (t as usize).div_ceil(size)
                }
            })
            .unwrap_or(0)
    });
    let has_prev = Signal::derive(move || page.get() > 1);
    let has_next = Signal::derive(move || page.get() < total_pages.get());
    let can_first = has_prev;
    let can_last = has_next;
    let page_info = Signal::derive(move || {
        let t = total.get().unwrap_or(0);
        let p = page.get();
        let size = batch_size.get();
        let total_pg = total_pages.get();
        if total_pg > 0 {
            let start = (p - 1) * size + 1;
            let end = (p * size).min(t as usize);
            t_string!(
                i18n,
                archive.page_info,
                page = p,
                total_pages = total_pg,
                start = start,
                end = end,
                total = t
            )
        } else {
            t_string!(i18n, archive.games_loaded_count, loaded = 0, total = t)
        }
    });
    let compact_page_info = Signal::derive(move || {
        let total_pg = total_pages.get();
        if total_pg > 0 {
            format!("{} / {}", page.get(), total_pg)
        } else {
            "0 / 0".to_owned()
        }
    });
    let top_pagination_ref = NodeRef::<html::Div>::new();
    let compact_pagination_visible = RwSignal::new(false);
    _ = use_intersection_observer_with_options(
        top_pagination_ref,
        move |entries, _| {
            if let Some(entry) = entries.first() {
                compact_pagination_visible
                    .set(!entry.is_intersecting() && entry.bounding_client_rect().top() < 0.0);
            }
        },
        UseIntersectionObserverOptions::default().thresholds(vec![0.01]),
    );

    view! {
        <div class="space-y-4">
            <Show when=move || total.with(|v| v.is_some())>
                <div node_ref=top_pagination_ref class="mx-auto w-full max-w-screen-2xl ui-panel">
                    <div class="py-3 px-3 sm:px-4">
                        <div class="grid gap-3 sm:flex sm:flex-wrap sm:gap-4 sm:justify-between sm:items-center">
                            <p class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                {page_info}
                            </p>
                            <div class="grid gap-3 sm:flex sm:flex-wrap sm:items-center">
                                <div class="flex flex-wrap gap-2 justify-between items-center sm:order-1 sm:gap-3 sm:justify-start">
                                    <Show when=move || {
                                        draft_options.is_some() && on_search.is_some()
                                    }>
                                        <div class="flex gap-2 items-center">
                                            <label class="text-sm font-medium text-gray-700 whitespace-nowrap dark:text-gray-300 max-[285px]:text-xs">
                                                {t!(i18n, archive.page_size)}
                                            </label>
                                            <select
                                                class="w-16 h-10 ui-field-select min-h-10 xs:w-20 xs:h-11 xs:min-h-11"
                                                prop:value=Signal::derive(move || {
                                                    batch_size.get().to_string()
                                                })
                                                on:change=move |ev| {
                                                    let val = event_target_value(&ev);
                                                    if let (Some(opts), Some(search)) = (
                                                        draft_options.as_ref(),
                                                        on_search.as_ref(),
                                                    ) {
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
                                                            <option value=size.to_string()>{size.to_string()}</option>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </select>
                                        </div>
                                    </Show>
                                    <button
                                        type="button"
                                        class=copy_button_class
                                        aria-label=move || {
                                            t_string!(i18n, archive.permalink).to_string()
                                        }
                                        on:click=copy_permalink
                                    >
                                        <Icon
                                            icon=Signal::derive(move || {
                                                if copy_state.get() {
                                                    icondata_ai::AiCheckOutlined
                                                } else {
                                                    icondata_ai::AiCopyOutlined
                                                }
                                            })
                                            attr:class="size-4"
                                        />
                                        {move || {
                                            if copy_state.get() {
                                                t_string!(i18n, archive.copied).to_string()
                                            } else {
                                                t_string!(i18n, archive.permalink).to_string()
                                            }
                                        }}
                                    </button>
                                </div>
                                <div class="flex gap-1 justify-center w-full min-w-0 sm:order-2 sm:justify-start sm:w-auto max-[285px]:gap-0.5 xs:gap-1.5">
                                    <button
                                        type="button"
                                        class=PAGINATION_BTN_CLASS
                                        disabled=move || !can_first.get()
                                        aria-label=move || {
                                            t_string!(i18n, archive.first_page).to_string()
                                        }
                                        on:click=move |_| {
                                            if can_first.get_untracked() {
                                                on_page_change.run(1);
                                            }
                                        }
                                    >
                                        <Icon
                                            icon=icondata_ai::AiFastBackwardFilled
                                            attr:class="size-5"
                                        />
                                    </button>
                                    <button
                                        type="button"
                                        class=PAGINATION_BTN_CLASS
                                        disabled=move || !has_prev.get()
                                        aria-label=move || {
                                            t_string!(i18n, archive.prev_page).to_string()
                                        }
                                        on:click=move |_| {
                                            if has_prev.get_untracked() {
                                                on_page_change.run(page.get_untracked() - 1);
                                            }
                                        }
                                    >
                                        <Icon
                                            icon=icondata_ai::AiStepBackwardFilled
                                            attr:class="size-5"
                                        />
                                    </button>
                                    <button
                                        type="button"
                                        class=PAGINATION_BTN_CLASS
                                        disabled=move || !has_next.get()
                                        aria-label=move || {
                                            t_string!(i18n, archive.next_page).to_string()
                                        }
                                        on:click=move |_| {
                                            if has_next.get_untracked() {
                                                on_page_change.run(page.get_untracked() + 1);
                                            }
                                        }
                                    >
                                        <Icon
                                            icon=icondata_ai::AiStepForwardFilled
                                            attr:class="size-5"
                                        />
                                    </button>
                                    <button
                                        type="button"
                                        class=PAGINATION_BTN_CLASS
                                        disabled=move || !can_last.get()
                                        aria-label=move || {
                                            t_string!(i18n, archive.last_page).to_string()
                                        }
                                        on:click=move |_| {
                                            if can_last.get_untracked() {
                                                on_page_change.run(total_pages.get_untracked());
                                            }
                                        }
                                    >
                                        <Icon
                                            icon=icondata_ai::AiFastForwardFilled
                                            attr:class="size-5"
                                        />
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
            <Show when=move || total.get().is_some() && compact_pagination_visible.get()>
                <div class="fixed inset-x-0 bottom-3 z-40 px-3 pointer-events-none sm:bottom-4 lg:top-12 lg:bottom-auto">
                    <div class="flex gap-2 items-center py-2 px-3 mx-auto rounded-lg border ring-1 shadow-xl pointer-events-auto w-fit max-w-[calc(100vw-1.5rem)] border-black/10 bg-even-light/95 ring-black/5 backdrop-blur dark:border-white/10 dark:bg-surface-panel/95 dark:ring-white/10">
                        <span class="text-sm font-semibold tabular-nums text-gray-700 whitespace-nowrap dark:text-gray-200">
                            {compact_page_info}
                        </span>
                        <div class="flex gap-1 items-center">
                            <button
                                type="button"
                                class=COMPACT_PAGINATION_BTN_CLASS
                                disabled=move || !can_first.get()
                                aria-label=move || {
                                    t_string!(i18n, archive.first_page).to_string()
                                }
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
                                class=COMPACT_PAGINATION_BTN_CLASS
                                disabled=move || !has_prev.get()
                                aria-label=move || {
                                    t_string!(i18n, archive.prev_page).to_string()
                                }
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
                                class=COMPACT_PAGINATION_BTN_CLASS
                                disabled=move || !has_next.get()
                                aria-label=move || {
                                    t_string!(i18n, archive.next_page).to_string()
                                }
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
                                class=COMPACT_PAGINATION_BTN_CLASS
                                disabled=move || !can_last.get()
                                aria-label=move || {
                                    t_string!(i18n, archive.last_page).to_string()
                                }
                                on:click=move |_| {
                                    if can_last.get_untracked() {
                                        on_page_change.run(total_pages.get_untracked());
                                    }
                                }
                            >
                                <Icon icon=icondata_ai::AiFastForwardFilled attr:class="size-5" />
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
            <div class="pb-20 mx-auto space-y-4 w-full max-w-screen-2xl">
                <Show when=has_searched>
                    <div class="flex flex-col">
                        <div class="gap-3 min-h-0 rounded-lg sm:grid sm:grid-cols-2 sm:gap-4 sm:content-start lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
                            <For each=games key=|game| game.game_id.clone() let:game>
                                <GameRow game />
                            </For>
                        </div>
                        <Show when=move || games.with(|g| g.is_empty()) && !is_loading.get()>
                            <EmptyState
                                title=move || { t_string!(i18n, archive.no_games_found) }
                                class="mt-6"
                            />
                        </Show>
                        <Show when=is_loading>
                            <EmptyState
                                title=move || { t_string!(i18n, archive.loading_games) }
                                class="mt-6 animate-pulse"
                            />
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    }
}
