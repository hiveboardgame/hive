use crate::{
    common::focus_after_render,
    components::molecules::{
        pagination_controls::PaginationControls,
        tournament_entrant_suggestions::TournamentEntrantSuggestions,
    },
    i18n::*,
};
use leptos::{html, prelude::*};
use leptos_icons::Icon;
use uuid::Uuid;

fn focus_overview_standing(player: Uuid, mounted: ArcRwSignal<bool>) {
    let selector = format!(
        "[data-tournament-overview-standing=\"true\"][data-tournament-player-id=\"{player}\"], [data-tournament-overview-standing=\"true\"][data-arena-player-id=\"{player}\"]",
    );
    focus_after_render(selector, mounted);
}

#[component]
pub fn TournamentStandingsControls(
    page: Signal<usize>,
    total: Signal<usize>,
    #[prop(into)] page_size: Signal<usize>,
    query: Signal<String>,
    on_query_change: Callback<String>,
    on_page_change: Callback<usize>,
    #[prop(optional)] display_total: Option<Signal<usize>>,
    #[prop(optional)] struck_total: Option<Signal<Option<usize>>>,
    #[prop(optional)] me_player: Option<Signal<Option<Uuid>>>,
    #[prop(optional)] me_on_page: Option<Signal<bool>>,
    #[prop(optional)] on_me: Option<Callback<Uuid>>,
    #[prop(optional)] show_pagination_when_empty: bool,
    #[prop(optional)] search_only: bool,
    entrant_names: Signal<Vec<String>>,
    suggestions_id: String,
) -> impl IntoView {
    let i18n = use_i18n();
    let display_total = display_total.unwrap_or(total);
    let struck_total = struck_total.unwrap_or_else(|| Signal::derive(|| None));
    let searching = RwSignal::new(false);
    let input_ref = NodeRef::<html::Input>::new();
    let mounted = ArcRwSignal::new(true);
    let cleanup_mounted = mounted.clone();
    on_cleanup(move || cleanup_mounted.set(false));

    Effect::new(move |_| {
        if searching.get() {
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        }
    });

    view! {
        <div class="flex flex-wrap gap-1 items-center min-w-0">
            <Show when=move || !search_only>
                <button
                    type="button"
                    class="inline-flex justify-center items-center rounded size-10 shrink-0 ui-button ui-button-ghost"
                    aria-label=move || {
                        if searching.get() {
                            t_string!(i18n, tournaments.view.arena.close).to_string()
                        } else {
                            t_string!(i18n, archive.search).to_string()
                        }
                    }
                    aria-pressed=move || searching.get().to_string()
                    on:click=move |_| {
                        let next = !searching.get_untracked();
                        searching.set(next);
                        if !next {
                            on_query_change.run(String::new());
                            on_page_change.run(1);
                        }
                    }
                >
                    <Show
                        when=move || searching.get()
                        fallback=|| {
                            view! { <Icon icon=icondata_io::IoSearch attr:class="size-5" /> }
                        }
                    >
                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-5" />
                    </Show>
                </button>
            </Show>
            <Show when=move || search_only || searching.get()>
                <form
                    class="flex gap-1 items-center min-w-0"
                    on:submit=move |event| event.prevent_default()
                >
                    <input
                        node_ref=input_ref
                        class="min-w-0 h-10 max-w-52 ui-field-input"
                        type="search"
                        list=suggestions_id.clone()
                        // TODO: i18n once copy is approved.
                        placeholder=move || {
                            if search_only {
                                String::from("Find a player")
                            } else {
                                t_string!(i18n, archive.search).to_string()
                            }
                        }
                        aria-label=move || t_string!(i18n, archive.search).to_string()
                        prop:value=move || query.get()
                        on:input=move |event| on_query_change.run(event_target_value(&event))
                    />
                    <TournamentEntrantSuggestions id=suggestions_id.clone() entrant_names />
                </form>
            </Show>
            <Show when=move || !search_only>
                <PaginationControls
                    page
                    total
                    page_size
                    on_page_change
                    display_total
                    struck_total
                    show_when_empty=show_pagination_when_empty
                />
            </Show>
            <Show when=move || {
                on_me.is_some() && me_player.is_some_and(|me_player| me_player.get().is_some())
                    && !me_on_page.is_some_and(|me_on_page| me_on_page.get())
            }>
                // TODO: i18n once copy is approved.
                <button
                    type="button"
                    class="inline-flex justify-center items-center rounded size-10 shrink-0 ui-button ui-button-ghost"
                    aria-label="Find your standing"
                    on:click={
                        let mounted = mounted.clone();
                        move |_| {
                            let Some(player) = me_player
                                .and_then(|me_player| me_player.get_untracked()) else {
                                return;
                            };
                            searching.set(false);
                            on_query_change.run(String::new());
                            if let Some(on_me) = on_me {
                                on_me.run(player);
                                focus_overview_standing(player, mounted.clone());
                            }
                        }
                    }
                >
                    <Icon icon=icondata_ai::AiAimOutlined attr:class="size-5" />
                </button>
            </Show>
        </div>
    }
}
