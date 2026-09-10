use crate::{
    common::{format_local_datetime, with_class},
    components::molecules::{empty_state::EmptyState, upcoming_game_row::UpcomingGameRow},
    functions::schedules::get_upcoming_tournament_games,
    i18n::*,
};
use chrono::{DateTime, Duration, Local};
use leptos::prelude::*;
use leptos_use::{
    use_interval_fn_with_options,
    utils::Pausable,
    watch_with_options,
    UseIntervalFnOptions,
    WatchOptions,
};

#[component]
pub fn Calendar() -> impl IntoView {
    let i18n = use_i18n();
    let upcoming_games = OnceResource::new(get_upcoming_tournament_games());
    let last_updated = RwSignal::new(None::<DateTime<Local>>);
    let current_time = RwSignal::new(Local::now());

    let should_run_timer = Signal::derive(move || {
        upcoming_games.with(|games| {
            games
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .and_then(|games| games.first())
                .is_some_and(|(start_time, _)| {
                    let local_start = start_time.with_timezone(&Local);
                    let time_until_start = local_start.signed_duration_since(current_time.get());
                    time_until_start <= Duration::minutes(60)
                        && time_until_start > Duration::minutes(-10)
                })
        })
    });

    let Pausable { pause, resume, .. } = use_interval_fn_with_options(
        move || current_time.set(Local::now()),
        60_000, // 1 minute intervals
        UseIntervalFnOptions::default().immediate(false),
    );

    let _ = watch_with_options(
        should_run_timer,
        move |should_run, _, _| {
            if *should_run {
                resume();
            } else {
                pause();
            }
        },
        WatchOptions::default().immediate(true),
    );

    Effect::new(move |_| {
        if upcoming_games.with(|r| r.is_some()) {
            last_updated.set(Some(Local::now()));
        }
    });

    view! {
        <div class="overflow-hidden w-full ui-panel">
            <div class=with_class(
                "ui-panel-header",
                "sticky top-0 z-10 flex-col justify-center text-center",
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
            <div class="pt-3 ui-panel-body">

                <Suspense fallback=move || {
                    view! {
                        <EmptyState
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
                            <EmptyState title=move || {
                                t_string!(i18n, tournaments.calendar.error).to_string()
                            } />
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
