use leptos::*;
use leptos::prelude::*;
use std::collections::HashMap;
use crate::{
    functions::games::get::{
        get_site_statistics_games_by_type,
        get_site_statistics_winrate_by_rating_difference,
    },
    functions::users::get_statistics_number_user_registrations,
    components::molecules::banner::Banner,
};
use shared_types::SiteStatisticsTimePeriod;
struct TableRow {
    label: String,
    values: Vec<i64>,
    additional_values: Option<Vec<f64>>,
}

const BASE_SELECTOR_STYLE: &str = "no-link-style py-1 px-2 text-sm font-semibold rounded-lg border-2 transition-all duration-200 transform hover:scale-[1.02] cursor-pointer shadow-sm hover:shadow-md";
const ACTIVE_SELECTOR_STYLE: &str = "bg-pillbug-teal border-pillbug-teal text-white hover:bg-pillbug-teal/90";
const INACTIVE_SELECTOR_STYLE: &str = "bg-gray-50 border-gray-200 text-gray-700 hover:bg-gray-100 hover:border-gray-300 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:border-gray-500";

#[component]
pub fn Statistics() -> impl IntoView {
    let (selected_speed, set_selected_speed) = signal("All".to_string());

    let (selected_period, set_selected_period) = signal(SiteStatisticsTimePeriod::default());

    let (include_bots, set_include_bots) = signal(false);

    let statistics_games_by_type = Resource::new(
        move || (selected_period.get(), include_bots.get()),
        |(period, include_bots)| async move {
            get_site_statistics_games_by_type(period.to_string(), include_bots).await
        }
    );

    let site_statistics_winrate_by_rating_difference = Resource::new(
        move || (selected_period.get(), include_bots.get()),
        |(period, include_bots)| async move {
            get_site_statistics_winrate_by_rating_difference(period.to_string(), include_bots)
                .await
        }
    );

    let statistics_number_user_registrations = Resource::new(
        move || (selected_period.get(), include_bots.get()),
        |(period, include_bots)| async move {
            get_statistics_number_user_registrations(period.to_string(), include_bots)
                .await
                .ok()
                .unwrap_or(0)
        }
    );
    
    view! {
        <div class="flex flex-col items-center px-4 mx-auto w-full max-w-4xl sm:px-6 lg:px-8 pt-20">
            <Banner title="Site statistics" extend_tw_classes="w-10/12" />
            <PeriodSelector
                selected_period=selected_period
                set_selected_period=set_selected_period
            />
            <IncludeBotGamesSelector include_bots=include_bots set_include_bots=set_include_bots />

            <section class="space-y-4 mx-auto w-full max-w-4xl sm:px-6 lg:px-8">
                <Suspense fallback=move || {
                    view! { <div class="text-center py-4">"Loading site statistics..."</div> }
                }>
                    {move || {
                        statistics_games_by_type
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(stats) => {
                                        let total_games: i64 = stats.iter().map(|s| s.total).sum();
                                        let total_rated: i64 = stats
                                            .iter()
                                            .filter_map(|s| s.rated_games)
                                            .sum();
                                        let total_casual: i64 = stats
                                            .iter()
                                            .filter_map(|s| s.casual_games)
                                            .sum();
                                        let total_tournament: i64 = stats
                                            .iter()
                                            .filter_map(|s| s.tournament_games)
                                            .sum();
                                        let user_count = statistics_number_user_registrations
                                            .get()
                                            .unwrap_or(0);
                                        let games_by_type_and_time_control: Vec<TableRow> = stats
                                            .iter()
                                            .map(|s| TableRow {
                                                label: s.speed.clone(),
                                                values: vec![
                                                    s.rated_games.unwrap_or(0) + s.casual_games.unwrap_or(0),
                                                    s.rated_games.unwrap_or(0),
                                                    s.tournament_games.unwrap_or(0),
                                                    s.casual_games.unwrap_or(0),
                                                ],
                                                additional_values: None,
                                            })
                                            .collect();

                                        view! {
                                            <div class="space-y-4 text-center">
                                                <SingleStatCard
                                                    label="Players registered"
                                                    value=user_count
                                                />
                                                <div class="grid grid-cols-2 md:grid-cols-4 gap-4 ">
                                                    <SingleStatCard label="Games played" value=total_games />
                                                    <SingleStatCard label="Rated" value=total_rated />
                                                    <SingleStatCard label="Casual" value=total_casual />
                                                    <SingleStatCard label="Tournament" value=total_tournament />
                                                </div>
                                                <div class="grid md:grid-cols-1">
                                                    <StatTableCardGeneric
                                                        title="Games by Type and Time Control"
                                                        headers=vec![
                                                            "Time Control",
                                                            "Total",
                                                            "Rated",
                                                            "Tournament",
                                                            "Casual",
                                                        ]
                                                        rows=games_by_type_and_time_control
                                                    />
                                                </div>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    Err(e) => {
                                        view! {
                                            <div class="text-center py-4 text-red-500">
                                                "Error loading statistics: " {e.to_string()}
                                            </div>
                                        }
                                            .into_any()
                                    }
                                }
                            })
                    }}
                </Suspense>
            </section>
            <section class="space-y-4 mx-auto w-full max-w-4xl sm:px-6 lg:px-8">
                <Suspense fallback=move || {
                    view! { <div class="text-center py-4 pt-10">"Loading site statistics..."</div> }
                }>
                    {move || {
                        site_statistics_winrate_by_rating_difference
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(stats) => {
                                        let speeds: Vec<String> = {
                                            let mut unique_speeds: Vec<String> = stats
                                                .iter()
                                                .map(|s| s.speed.clone())
                                                .collect::<std::collections::HashSet<_>>()
                                                .into_iter()
                                                .collect();
                                            unique_speeds.sort();
                                            unique_speeds
                                        };
                                        let filtered_stats: Vec<_> = if selected_speed.get()
                                            == "All"
                                        {
                                            stats.clone()
                                        } else {
                                            stats
                                                .iter()
                                                .filter(|s| s.speed == selected_speed.get())
                                                .cloned()
                                                .collect()
                                        };
                                        let total_games: i64 = filtered_stats
                                            .iter()
                                            .map(|s| s.num_games)
                                            .sum();
                                        let total_white_wins: i64 = filtered_stats
                                            .iter()
                                            .filter(|s| s.game_status == "Finished(1-0)")
                                            .map(|s| s.num_games)
                                            .sum();
                                        let total_black_wins: i64 = filtered_stats
                                            .iter()
                                            .filter(|s| s.game_status == "Finished(0-1)")
                                            .map(|s| s.num_games)
                                            .sum();
                                        let total_draws: i64 = filtered_stats
                                            .iter()
                                            .filter(|s| s.game_status == "Finished(½-½)")
                                            .map(|s| s.num_games)
                                            .sum();
                                        let white_wins_share = if total_games > 0 {
                                            (total_white_wins as f64 / total_games as f64) * 100.0
                                        } else {
                                            0.0
                                        };
                                        let black_wins_share = if total_games > 0 {
                                            (total_black_wins as f64 / total_games as f64) * 100.0
                                        } else {
                                            0.0
                                        };
                                        let draws_share = if total_games > 0 {
                                            (total_draws as f64 / total_games as f64) * 100.0
                                        } else {
                                            0.0
                                        };
                                        let mut bucket_map: HashMap<String, (i64, i64, i64)> = HashMap::new();
                                        for stat in &filtered_stats {
                                            let entry = bucket_map
                                                .entry(stat.bucket.clone())
                                                .or_insert((0, 0, 0));
                                            match stat.game_status.as_str() {
                                                "Finished(1-0)" => entry.0 += stat.num_games,
                                                "Finished(0-1)" => entry.1 += stat.num_games,
                                                "Finished(½-½)" => entry.2 += stat.num_games,
                                                _ => {}
                                            }
                                        }
                                        let bucket_order = vec![
                                            "White > 300+",
                                            "White > 200",
                                            "White > 100",
                                            "White > 0",
                                            "Black > 0",
                                            "Black > 100",
                                            "Black > 200",
                                            "Black > 300+",
                                        ];
                                        let wins_by_color_and_rating_difference: Vec<TableRow> = bucket_order
                                            .iter()
                                            .filter_map(|bucket| {
                                                bucket_map
                                                    .get(*bucket)
                                                    .map(|(white_wins, black_wins, draws)| {
                                                        let bucket_total = white_wins + black_wins + draws;
                                                        let white_wins_share = if bucket_total > 0 {
                                                            (*white_wins as f64 / bucket_total as f64) * 100.0
                                                        } else {
                                                            0.0
                                                        };
                                                        let black_wins_share = if bucket_total > 0 {
                                                            (*black_wins as f64 / bucket_total as f64) * 100.0
                                                        } else {
                                                            0.0
                                                        };
                                                        let draws_share = if bucket_total > 0 {
                                                            (*draws as f64 / bucket_total as f64) * 100.0
                                                        } else {
                                                            0.0
                                                        };
                                                        TableRow {
                                                            label: bucket.to_string(),
                                                            values: vec![*white_wins, *black_wins, *draws],
                                                            additional_values: Some(
                                                                vec![white_wins_share, black_wins_share, draws_share],
                                                            ),
                                                        }
                                                    })
                                            })
                                            .collect();

                                        view! {
                                            <div class="space-y-4 text-center pt-10">
                                                <div class="flex gap-2 mb-4 flex-wrap justify-center">
                                                    <button
                                                        type="button"
                                                        class=move || {
                                                            if selected_speed.get() == "All" {
                                                                format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                                            } else {
                                                                format!(
                                                                    "{} {}",
                                                                    BASE_SELECTOR_STYLE,
                                                                    INACTIVE_SELECTOR_STYLE,
                                                                )
                                                            }
                                                        }
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            set_selected_speed.set("All".to_string());
                                                        }
                                                    >
                                                        "All speeds"
                                                    </button>
                                                    {speeds
                                                        .iter()
                                                        .map(|speed| {
                                                            let speed_clone = speed.clone();
                                                            let speed_clone2 = speed.clone();
                                                            let speed_display = speed.clone();
                                                            view! {
                                                                <button
                                                                    type="button"
                                                                    class=move || {
                                                                        if selected_speed.get() == speed_clone {
                                                                            format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                                                        } else {
                                                                            format!(
                                                                                "{} {}",
                                                                                BASE_SELECTOR_STYLE,
                                                                                INACTIVE_SELECTOR_STYLE,
                                                                            )
                                                                        }
                                                                    }
                                                                    on:click=move |e| {
                                                                        e.prevent_default();
                                                                        set_selected_speed.set(speed_clone2.clone());
                                                                    }
                                                                >
                                                                    {speed_display}
                                                                </button>
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}
                                                </div>

                                                <div class="grid grid-cols-3 md:grid-cols-3 gap-4">
                                                    <SingleStatCard
                                                        label="White wins"
                                                        value=total_white_wins
                                                        additional_value=white_wins_share
                                                    />
                                                    <SingleStatCard
                                                        label="Black wins"
                                                        value=total_black_wins
                                                        additional_value=black_wins_share
                                                    />
                                                    <SingleStatCard
                                                        label="Draws"
                                                        value=total_draws
                                                        additional_value=draws_share
                                                    />
                                                </div>
                                                <div class="grid md:grid-cols-1">
                                                    <StatTableCardGeneric
                                                        title="Wins by color and rating difference"
                                                        headers=vec![
                                                            "Rating Difference",
                                                            "White wins",
                                                            "Black wins",
                                                            "Draws",
                                                        ]
                                                        rows=wins_by_color_and_rating_difference
                                                    />
                                                </div>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    Err(e) => {
                                        view! {
                                            <div class="text-center py-4 text-red-500">
                                                "Error loading statistics: " {e.to_string()}
                                            </div>
                                        }
                                            .into_any()
                                    }
                                }
                            })
                    }}
                </Suspense>
            </section>
        </div>
    }
}

#[component]
fn PeriodSelector(
    selected_period: ReadSignal<SiteStatisticsTimePeriod>,
    set_selected_period: WriteSignal<SiteStatisticsTimePeriod>,
) -> impl IntoView {
    let periods = SiteStatisticsTimePeriod::all();

    view! {
        <div class="flex gap-2 mb-4 flex-wrap">
            {periods
                .into_iter()
                .map(|period| {
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if selected_period.get() == period {
                                    format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                } else {
                                    format!("{} {}", BASE_SELECTOR_STYLE, INACTIVE_SELECTOR_STYLE)
                                }
                            }
                            on:click=move |e| {
                                e.prevent_default();
                                set_selected_period.set(period)
                            }
                        >
                            {period.to_string()}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
fn IncludeBotGamesSelector(
    include_bots: ReadSignal<bool>,
    set_include_bots: WriteSignal<bool>,
) -> impl IntoView {
    let include_bots_vec = vec![
        (true, "Include Bots"),
        (false, "Exclude Bots"),
    ];

    view! {
        <div class="flex gap-2 mb-4 flex-wrap">
            {include_bots_vec
                .into_iter()
                .map(|(value, label)| {
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if include_bots.get() == value {
                                    format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                } else {
                                    format!("{} {}", BASE_SELECTOR_STYLE, INACTIVE_SELECTOR_STYLE)
                                }
                            }
                            on:click=move |e| {
                                e.prevent_default();
                                set_include_bots.set(value)
                            }
                        >
                            {label}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
fn SingleStatCard(
    label: &'static str, 
    value: i64,
    #[prop(optional)] additional_value: Option<f64>
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-700 rounded-2xl shadow p-4">
            <div class="text-2xl font-bold">{value.to_string()}</div>
            {additional_value
                .map(|add_val| {
                    view! {
                        <div class="text-m text-gray-500 ml-1">{format!("{:.1}", add_val)} "%"</div>
                    }
                })}
            <div class="text-sm text-gray-500">{label}</div>
        </div>
    }
}

#[component]
fn StatTableCardGeneric(
    title: &'static str,
    headers: Vec<&'static str>,
    rows: Vec<TableRow>,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-700 rounded-2xl shadow p-4">
            <h3 class="font-semibold mb-2">{title}</h3>
            <table class="w-full text-sm">
                <thead class="border-b border-gray-300">
                    <tr>
                        {headers
                            .into_iter()
                            .enumerate()
                            .map(|(i, h)| {
                                view! {
                                    <th class=move || {
                                        if i == 0 { "py-1" } else { "py-1 text-center" }
                                    }>{h}</th>
                                }
                            })
                            .collect::<Vec<_>>()}
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|row| {
                            let additional_values = row.additional_values.clone();
                            view! {
                                <tr class="border-b border-gray-200 last:border-none">
                                    <td class="py-1">{row.label}</td>
                                    {row
                                        .values
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, value)| {
                                            let additional = additional_values
                                                .as_ref()
                                                .and_then(|av| av.get(i))
                                                .copied();

                                            view! {
                                                <td class="py-1 text-center">
                                                    <div>{value.to_string()}</div>
                                                    {additional
                                                        .map(|add_val| {
                                                            view! {
                                                                <div class="text-xs text-gray-500">
                                                                    {format!("{:.1}", add_val)} "%"
                                                                </div>
                                                            }
                                                        })}
                                                </td>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </tr>
                            }
                        })
                        .collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    }
}