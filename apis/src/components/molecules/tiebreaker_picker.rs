use crate::{common::tiebreaker_explanation, i18n::*};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{Tiebreaker, TournamentMode};

/// Picks the tiebreakers a tournament ranks by, and the order they apply in.
///
/// Order is the whole point: the first one that separates two players decides,
/// so moving Buchholz above Sonneborn-Berger is a different tournament. The
/// list starts from the mode's own sensible defaults and the organizer adjusts
/// from there.
#[component]
pub fn TiebreakerPicker(
    mode: Signal<TournamentMode>,
    selected: RwSignal<Vec<Option<Tiebreaker>>>,
) -> impl IntoView {
    let engine_ranks_itself = Signal::derive(move || match mode.get() {
        TournamentMode::Arena => {
            Some("Arena automatically ranks by points, then wins, then fewest games")
        }
        mode if mode.is_elimination() => {
            Some("A bracket automatically ranks by how far each player got")
        }
        _ => None,
    });
    let i18n = use_i18n();
    let explain = move |tiebreaker| tiebreaker_explanation(i18n, tiebreaker);

    // Whenever the mode *changes* the previous choice may not even be
    // meaningful any more — a bracket cannot use Buchholz — so the list resets
    // to what the new mode ranks by.
    //
    // Deliberately not on the first run: this component is remounted whenever
    // the page is, and resetting then would throw away a selection the
    // organizer had already made. The form seeds its own defaults on arrival.
    Effect::new(move |previous: Option<TournamentMode>| {
        let mode = mode.get();
        if let Some(previous) = previous {
            if previous != mode {
                selected.set(
                    Tiebreaker::defaults_for(mode)
                        .into_iter()
                        .map(Some)
                        .collect(),
                );
            }
        }
        mode
    });

    let chosen = move || {
        selected
            .get()
            .into_iter()
            .flatten()
            .collect::<Vec<Tiebreaker>>()
    };
    let unused = move || {
        let chosen = chosen();
        Tiebreaker::available_for(mode.get())
            .into_iter()
            .filter(|tiebreaker| !chosen.contains(tiebreaker))
            .collect::<Vec<_>>()
    };

    let move_up = move |index: usize| {
        selected.update(|list| {
            if index > 0 {
                list.swap(index - 1, index);
            }
        });
    };
    let remove = move |index: usize| {
        selected.update(|list| {
            list.remove(index);
        });
    };
    let add = move |tiebreaker: Tiebreaker| {
        selected.update(|list| list.push(Some(tiebreaker)));
    };

    let engine_ranked = Signal::derive(move || Tiebreaker::available_for(mode.get()).is_empty());
    let numbered = move || chosen().into_iter().enumerate().collect::<Vec<_>>();

    view! {
        <div class="ui-setting-group">
            <span class="ui-field-label">"Tiebreakers"</span>
            <Show
                when=move || !engine_ranked.get()
                fallback=move || {
                    view! { <small class="ui-field-helper">{engine_ranks_itself.get()}</small> }
                }
            >
                <small class="ui-field-helper">
                    "Applied in order. The first one that separates two players decides."
                </small>
                <ol class="flex flex-col gap-1 mt-2">
                    <For each=numbered key=|(index, tiebreaker)| (*index, *tiebreaker) let:entry>
                        {
                            let (index, tiebreaker) = entry;
                            view! {
                                <li class="flex gap-2 items-center">
                                    <span class="w-6 text-sm text-gray-500">
                                        {format!("{}.", index + 1)}
                                    </span>
                                    <span
                                        class="flex flex-col min-w-0 grow"
                                        title=explain(tiebreaker)
                                    >
                                        <span class="text-sm">{tiebreaker.full_name()}</span>
                                        <span class="text-xs text-gray-500 dark:text-gray-400">
                                            {explain(tiebreaker)}
                                        </span>
                                    </span>
                                    <button
                                        type="button"
                                        title="Apply this one earlier"
                                        class="ui-button ui-button-secondary ui-button-icon"
                                        prop:disabled=index == 0
                                        on:click=move |_| move_up(index)
                                    >
                                        <Icon
                                            icon=icondata_ai::AiArrowUpOutlined
                                            attr:class="size-4"
                                        />
                                    </button>
                                    <button
                                        type="button"
                                        title="Stop using this tiebreaker"
                                        class="ui-button ui-button-danger ui-button-icon"
                                        on:click=move |_| remove(index)
                                    >
                                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                                    </button>
                                </li>
                            }
                        }
                    </For>
                </ol>
                <div class="flex flex-wrap gap-1 mt-2">
                    <For each=unused key=|tiebreaker| *tiebreaker let:tiebreaker>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            title=explain(tiebreaker)
                            on:click=move |_| add(tiebreaker)
                        >
                            {format!("+ {}", tiebreaker.full_name())}
                        </button>
                    </For>
                </div>
            </Show>
        </div>
    }
}
