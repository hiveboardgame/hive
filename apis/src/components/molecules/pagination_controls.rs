use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;

const BUTTON_CLASS: &str = "inline-flex size-10 items-center justify-center rounded ui-button ui-button-ghost disabled:cursor-not-allowed disabled:opacity-35";

fn page_range(page: usize, total: usize, page_size: usize) -> Option<(usize, usize)> {
    if total == 0 || page_size == 0 {
        return None;
    }
    let page = page.max(1);
    let start = (page - 1)
        .saturating_mul(page_size)
        .saturating_add(1)
        .min(total);
    let end = page.saturating_mul(page_size).min(total);
    Some((start, end))
}

#[component]
pub fn PaginationControls(
    page: Signal<usize>,
    total: Signal<usize>,
    page_size: usize,
    on_page_change: Callback<usize>,
    #[prop(optional)] display_total: Option<Signal<usize>>,
    #[prop(optional)] struck_total: Option<Signal<Option<usize>>>,
    #[prop(optional)] show_when_empty: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let display_total = display_total.unwrap_or(total);
    let struck_total = struck_total.unwrap_or_else(|| Signal::derive(|| None));
    let total_pages = Signal::derive(move || total.get().div_ceil(page_size).max(1));
    let has_previous = Signal::derive(move || page.get() > 1);
    let has_next = Signal::derive(move || page.get() < total_pages.get());
    let visible_range = Signal::derive(move || {
        let total = total.get();
        page_range(page.get(), total, page_size)
    });

    view! {
        <Show when=move || { show_when_empty || total.get() > 0 }>
            <nav
                class="flex gap-1 items-center"
                aria-label=move || t_string!(i18n, archive.pagination).to_string()
            >
                <div class="flex gap-0.5 items-center">
                    <button
                        type="button"
                        class=BUTTON_CLASS
                        disabled=move || !has_previous.get()
                        aria-label=move || t_string!(i18n, archive.first_page).to_string()
                        on:click=move |_| {
                            if has_previous.get_untracked() {
                                on_page_change.run(1);
                            }
                        }
                    >
                        <Icon icon=icondata_ai::AiFastBackwardFilled attr:class="size-5" />
                    </button>
                    <button
                        type="button"
                        class=BUTTON_CLASS
                        disabled=move || !has_previous.get()
                        aria-label=move || t_string!(i18n, archive.prev_page).to_string()
                        on:click=move |_| {
                            if has_previous.get_untracked() {
                                on_page_change.run(page.get_untracked() - 1);
                            }
                        }
                    >
                        <Icon icon=icondata_ai::AiStepBackwardFilled attr:class="size-5" />
                    </button>
                    <span class="inline-flex gap-1 justify-center px-1 text-sm font-medium tabular-nums text-center text-gray-700 dark:text-gray-300 min-w-[6.5rem]">
                        {move || {
                            visible_range
                                .get()
                                .map(|(start, end)| {
                                    format!("{start}–{end} / {}", display_total.get())
                                })
                                .unwrap_or_else(|| format!("0–0 / {}", display_total.get()))
                        }} <Show when=move || struck_total.get().is_some()>
                            <del class="text-gray-500 dark:text-gray-400">
                                {move || {
                                    struck_total
                                        .get()
                                        .map(|total| total.to_string())
                                        .unwrap_or_default()
                                }}
                            </del>
                        </Show>
                    </span>
                    <button
                        type="button"
                        class=BUTTON_CLASS
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
                        class=BUTTON_CLASS
                        disabled=move || !has_next.get()
                        aria-label=move || t_string!(i18n, archive.last_page).to_string()
                        on:click=move |_| {
                            if has_next.get_untracked() {
                                on_page_change.run(total_pages.get_untracked());
                            }
                        }
                    >
                        <Icon icon=icondata_ai::AiFastForwardFilled attr:class="size-5" />
                    </button>
                </div>
            </nav>
        </Show>
    }
}
