use crate::components::molecules::upcoming_game_row::UpcomingGameRow;
use crate::functions::schedules::get_upcoming_tournament_games;
use chrono::{DateTime, Duration, Local};
use leptos::prelude::*;
use leptos_use::{
    use_interval_fn_with_options, utils::Pausable, watch_with_options, UseIntervalFnOptions,
    WatchOptions,
};

#[component]
pub fn Calendar() -> impl IntoView {
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
        <div class="pb-4 w-full rounded-lg">
            <div class="sticky top-0 z-10 mb-4 text-center bg-light dark:bg-gray-950">
                <h2 class="text-xl font-bold">"Matches"</h2>
                <div class="text-xs opacity-75">
                    {move || {
                        match last_updated.get() {
                            Some(timestamp) => {
                                format!("Last updated: {}", timestamp.format("%m/%d %I:%M %p"))
                            }
                            None => "Loading...".to_string(),
                        }
                    }}
                </div>
            </div>

            <Suspense fallback=move || {
                view! {
                    <div class="flex justify-center items-center p-8">
                        <div class="text-center">
                            <div class="mb-2 text-lg">"Loading upcoming games..."</div>
                            <div class="text-sm opacity-75">
                                "Please wait while we fetch the scheduled games"
                            </div>
                        </div>
                    </div>
                }
            }>
                <ErrorBoundary fallback=|_errors| {
                    view! {
                        <div class="flex justify-center items-center p-8">
                            <div class="text-center text-red-500">
                                <div class="mb-2 text-lg">"Error loading upcoming games"</div>
                            </div>
                        </div>
                    }
                }>
                    {move || {
                        upcoming_games
                            .get()
                            .map(|games_result| {
                                games_result
                                    .map(|games| {
                                        if games.is_empty() {
                                            view! {
                                                <div class="flex justify-center items-center p-8">
                                                    <div class="text-center">
                                                        <div class="mb-2 text-lg">
                                                            "No upcoming tournament games"
                                                        </div>
                                                        <div class="text-sm opacity-75">
                                                            "Check back later for scheduled games"
                                                        </div>
                                                    </div>
                                                </div>
                                            }
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
                                        }
                                    })
                            })
                    }}
                </ErrorBoundary>
            </Suspense>
        </div>
    }
}
