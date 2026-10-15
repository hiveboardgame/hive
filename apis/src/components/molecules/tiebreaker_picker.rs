use crate::{common::StandingsCriterionChoice, i18n::*};
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn TiebreakerPicker<C>(
    selected: RwSignal<Vec<C>>,
    #[prop(into)] available: Signal<Vec<C>>,
) -> impl IntoView
where
    C: StandingsCriterionChoice,
{
    let i18n = use_i18n();
    let add_open = RwSignal::new(false);
    let chosen = move || selected.get();
    let unused = move || {
        let chosen = chosen();
        available
            .get()
            .iter()
            .copied()
            .filter(|criterion| {
                !chosen.contains(criterion)
                    && !(criterion.is_seed() && chosen.iter().any(|chosen| chosen.is_seed()))
            })
            .collect::<Vec<_>>()
    };
    let move_up = move |criterion: C| {
        selected.update(|list| {
            if let Some(index) = list.iter().position(|current| *current == criterion) {
                if index > 0 {
                    list.swap(index - 1, index);
                }
            }
        });
    };
    let move_down = move |criterion: C| {
        selected.update(|list| {
            if let Some(index) = list.iter().position(|current| *current == criterion) {
                if index + 1 < list.len() {
                    list.swap(index, index + 1);
                }
            }
        });
    };
    let remove = move |criterion: C| {
        selected.update(|list| {
            if let Some(index) = list.iter().position(|current| *current == criterion) {
                list.remove(index);
            }
        });
    };
    let add = move |criterion: C| {
        selected.update(|list| {
            if available.with(|available| available.contains(&criterion))
                && !list.contains(&criterion)
                && !(criterion.is_seed() && list.iter().any(|chosen| chosen.is_seed()))
            {
                list.push(criterion);
            }
        })
    };

    view! {
        <div class="space-y-2">
            // TODO: i18n once copy is approved.
            <span class="ui-field-label">"Tiebreakers"</span>
            <Show when=move || selected.with(Vec::is_empty)>
                // TODO: i18n once copy is approved.
                <p class="ui-field-helper">"Equal scores share a place."</p>
            </Show>
            <ol class="flex flex-col gap-1 mt-2">
                <For each=chosen key=|criterion| *criterion let:criterion>
                    {
                        let index = move || {
                            selected
                                .get()
                                .iter()
                                .position(|current| *current == criterion)
                                .unwrap_or_default()
                        };
                        view! {
                            <li class="grid gap-y-1 gap-x-2 items-center py-2 grid-cols-[1.25rem_minmax(0,1fr)_auto]">
                                <span class="w-6 text-sm text-gray-500">
                                    {move || format!("{}.", index() + 1)}
                                </span>
                                <span class="min-w-0" title=move || criterion.explanation(i18n)>
                                    <span class="text-sm">{move || criterion.name(i18n)}</span>

                                </span>
                                <div class="flex gap-1 shrink-0">
                                    <button
                                        type="button"
                                        title=move || {
                                            t_string!(i18n, tournaments.tiebreakers.move_earlier)
                                        }
                                        class="ui-button ui-button-secondary ui-button-icon"
                                        prop:disabled=move || index() == 0
                                        on:click=move |_| move_up(criterion)
                                    >
                                        <Icon
                                            icon=icondata_ai::AiArrowUpOutlined
                                            attr:class="size-4"
                                        />
                                    </button>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-icon"
                                        // TODO: i18n once copy is approved.
                                        title="Move later"
                                        prop:disabled=move || index() + 1 == selected.with(Vec::len)
                                        on:click=move |_| move_down(criterion)
                                    >
                                        <Icon
                                            icon=icondata_ai::AiArrowDownOutlined
                                            attr:class="size-4"
                                        />
                                    </button>
                                    <button
                                        type="button"
                                        title=move || {
                                            t_string!(i18n, tournaments.tiebreakers.remove)
                                        }
                                        class="ui-button ui-button-danger ui-button-icon"
                                        on:click=move |_| remove(criterion)
                                    >
                                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                                    </button>
                                </div>
                                <span class="col-span-2 col-start-2 min-w-0 text-xs text-gray-500 dark:text-gray-400">
                                    {move || criterion.explanation(i18n)}
                                </span>
                            </li>
                        }
                    }
                </For>
            </ol>
            // TODO: i18n once copy is approved.
            <button
                type="button"
                class="ui-button ui-button-secondary ui-button-sm"
                prop:disabled=move || unused().is_empty()
                on:click=move |_| add_open.update(|open| *open = !*open)
            >
                "+ Add tiebreaker"
            </button>
            <Show when=add_open>
                <div class="flex flex-wrap gap-2 p-3 ui-setting-group">
                    <For each=unused key=|criterion| *criterion let:criterion>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            title=move || criterion.explanation(i18n)
                            on:click=move |_| {
                                add(criterion);
                                add_open.set(false);
                            }
                        >
                            {move || criterion.name(i18n)}
                        </button>
                    </For>
                </div>
            </Show>
        </div>
    }
}
