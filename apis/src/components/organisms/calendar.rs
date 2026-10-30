use crate::{
    common::{format_local_datetime, with_class},
    components::molecules::{empty_state::EmptyState, upcoming_game_row::UpcomingGameRow},
    functions::schedules::get_upcoming_tournament_games,
    i18n::*,
};
use chrono::{DateTime, Local};
use leptos::prelude::*;
use leptos_use::use_interval_fn;

#[component]
pub fn Calendar() -> impl IntoView {
    let i18n = use_i18n();
    let upcoming_games = OnceResource::new(get_upcoming_tournament_games());
    let last_updated = RwSignal::new(None::<DateTime<Local>>);
    let current_time = RwSignal::new(Local::now());

    // Later games must become actionable even when the first entry is overdue.
    use_interval_fn(move || current_time.set(Local::now()), 60_000);

    Effect::new(move |_| {
        if upcoming_games.with(|r| r.is_some()) {
            last_updated.set(Some(Local::now()));
        }
    });

    view! {
        <div class="overflow-hidden w-full ui-panel">
            <div class=with_class(
                "ui-panel-header",
                "sticky top-0 z-10 flex-col justify-center text-center !gap-0.5 !py-2",
            )>
                <h2 class="text-xl font-bold">{t!(i18n, tournaments.calendar.title)}</h2>
                <div class="text-xs opacity-75">
                    {move || {
                        match last_updated.get() {
                            Some(timestamp) => {
                                t_string!(
                                    i18n,
                                    tournaments.calendar.last_updated,
                                    time = format_local_datetime(
                                        i18n.get_locale(),
                                        timestamp.to_utc(),
                                    ),
                                )
                                    .to_string()
                            }
                            None => t_string!(i18n, tournaments.calendar.loading).to_string(),
                        }
                    }}
                </div>
            </div>
            <div class="ui-panel-body !p-2 sm:!p-3">

                <Suspense fallback=move || {
                    view! {
                        <EmptyState
                            class="!py-4"
                            title=move || {
                                t_string!(i18n, tournaments.calendar.loading_games).to_string()
                            }
                            message=move || {
                                t_string!(i18n, tournaments.calendar.loading_help).to_string()
                            }
                        />
                    }
                }>
                    <ErrorBoundary fallback=move |_errors| {
                        view! {
                            <EmptyState
                                class="!py-4"
                                title=move || {
                                    t_string!(i18n, tournaments.calendar.error).to_string()
                                }
                            />
                        }
                    }>
                        {move || {
                            upcoming_games
                                .get()
                                .map(move |games_result| {
                                    games_result
                                        .map(move |games| {
                                            if games.is_empty() {
                                                view! {
                                                    <EmptyState
                                                        class="!py-4"
                                                        title=t_string!(
                                                            i18n,
                                                            tournaments.calendar.empty
                                                        )
                                                            .to_string()
                                                        message=t_string!(
                                                            i18n,
                                                            tournaments.calendar.empty_help
                                                        )
                                                            .to_string()
                                                    />
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <div class="flex flex-col gap-2 rounded-lg">
                                                        <For
                                                            each=move || games.clone()
                                                            key=|game_data| game_data.1.uuid
                                                            let:game_data
                                                        >
                                                            <UpcomingGameRow game_data current_time />
                                                        </For>
                                                    </div>
                                                }
                                                    .into_any()
                                            }
                                        })
                                })
                        }}
                    </ErrorBoundary>
                </Suspense>
            </div>
        </div>
    }
}
