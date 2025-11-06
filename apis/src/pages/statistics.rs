use crate::{
    components::atoms::rating_buckets_chart::RatingBucketsChart,
    components::atoms::site_statistics_generic_card::{
        StatTableCardGeneric, TableRow, TableRowLabel,
    },
    components::atoms::site_statistics_selectors::{
        GamesTypesSelector, IncludeBotGamesSelector, PeriodSelector, SpeedSelector,
    },
    components::atoms::site_statistics_single_stat_card::SingleStatCard,
    components::atoms::sticky_controls::StickyControls,
    components::molecules::banner::Banner,
    functions::site_statistics::{
        get_first_moves_winrate, get_games_by_type, get_most_active_players_by_period,
        get_number_user_registrations, get_rating_buckets, get_winrate_by_rating_difference,
    },
};
use leptos::prelude::*;
use leptos_meta::Style;
use shared_types::{GameSpeed, GameTypeFilter, TimePeriod};
use std::collections::HashMap;

const SECTION_STYLE: &str = "mx-auto w-full max-w-4xl sm:px-6 lg:px-8 px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg border-stone-400 dark:border-slate-600";

#[component]
pub fn Statistics() -> impl IntoView {
    let selected_speed = RwSignal::new(GameSpeed::AllSpeeds);
    let selected_period = RwSignal::new(TimePeriod::default());
    let include_bots = RwSignal::new(false);
    let included_game_types = RwSignal::new(GameTypeFilter::default());

    let statistics_games_by_type = Resource::new(
        move || {
            (
                selected_period.get(),
                include_bots.get(),
                included_game_types.get(),
            )
        },
        |(period, include_bots, included_game_types)| async move {
            get_games_by_type(period.to_string(), include_bots, included_game_types).await
        },
    );

    let winrate_by_rating_difference = Resource::new(
        move || {
            (
                selected_period.get(),
                include_bots.get(),
                included_game_types.get(),
            )
        },
        |(period, include_bots, included_game_types)| async move {
            get_winrate_by_rating_difference(period.to_string(), include_bots, included_game_types)
                .await
        },
    );

    let statistics_number_user_registrations = Resource::new(
        move || (selected_period.get(), include_bots.get()),
        |(period, include_bots)| async move {
            get_number_user_registrations(period.to_string(), include_bots)
                .await
                .ok()
                .unwrap_or(0)
        },
    );

    let statistics_most_active_players_by_period = Resource::new(
        move || {
            (
                selected_period.get(),
                include_bots.get(),
                included_game_types.get(),
            )
        },
        |(period, include_bots, included_game_types)| async move {
            get_most_active_players_by_period(
                period.to_string(),
                10,
                include_bots,
                included_game_types,
            )
            .await
        },
    );

    let first_moves_winrate = Resource::new(
        move || {
            (
                selected_period.get(),
                include_bots.get(),
                included_game_types.get(),
            )
        },
        |(period, include_bots, included_game_types)| async move {
            get_first_moves_winrate(period.to_string(), include_bots, included_game_types).await
        },
    );

    let rating_buckets = Resource::new(
        move || include_bots.get(),
        |include_bots| async move { get_rating_buckets(include_bots).await },
    );

    view! {
        <Style>
            "
            ._chartistry_line {
                stroke: #155dfc;
            }
            ._chartistry_line_markers {
                stroke: #155dfc;
            }
            
            ._chartistry_bar {
                fill: #155dfc;
            }
            .dark ._chartistry_line {
                stroke: #fcc800;
            }
            .dark ._chartistry_line_markers {
                fill: #fcc800;
            }
            .dark ._chartistry_bar {
                fill: #fcc800;
            }
            .dark ._chartistry_grid_line_x {
                stroke: #1a181845;
            }
            .dark ._chartistry_grid_line_y {
                stroke: #1a181845;
            }
            ._chartistry_tooltip {
                font-family: 'Inter', system-ui, sans-serif !important;
                font-size: 14px !important;
                color: #111 !important;
                background-color: #fff !important;
                border-radius: 6px !important;
            }
            .dark ._chartistry_tooltip {
                color: #f5f5f5 !important;
                background-color: #1a1a1a !important;
                border-color: #333 !important;
            }
            "
        </Style>

        <StickyControls>
            <PeriodSelector selected_period=selected_period />
            <SpeedSelector speeds=GameSpeed::all_for_stats() selected_speed=selected_speed />
            <GamesTypesSelector included_game_types=included_game_types />
            <IncludeBotGamesSelector include_bots=include_bots />
        </StickyControls>

        <div class="flex flex-col items-center px-4 mx-auto w-full max-w-4xl sm:px-6 lg:px-8 pt-16">
            <Banner title="Site statistics" extend_tw_classes="w-10/12" />

            <section class=SECTION_STYLE>
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
                                        let (
                                            filtered_total,
                                            filtered_rated,
                                            filtered_tournament,
                                            filtered_casual,
                                        ) = if selected_speed.get() == GameSpeed::AllSpeeds {
                                            (total_games, total_rated, total_tournament, total_casual)
                                        } else {
                                            let filtered: Vec<_> = stats
                                                .iter()
                                                .filter(|s| s.speed == selected_speed.get())
                                                .collect();
                                            (
                                                filtered.iter().map(|s| s.total).sum(),
                                                filtered.iter().filter_map(|s| s.rated_games).sum(),
                                                filtered.iter().filter_map(|s| s.tournament_games).sum(),
                                                filtered.iter().filter_map(|s| s.casual_games).sum(),
                                            )
                                        };

                                        view! {
                                            <div class="space-y-4 text-center">
                                                <SingleStatCard
                                                    label="Players registered"
                                                    value=user_count
                                                />
                                                <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                                                    <SingleStatCard label="Games played" value=filtered_total />
                                                    <SingleStatCard label="Rated" value=filtered_rated />
                                                    <SingleStatCard
                                                        label="Tournament"
                                                        value=filtered_tournament
                                                    />
                                                    <SingleStatCard label="Casual" value=filtered_casual />
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

            <section class=SECTION_STYLE>
                <Suspense fallback=move || {
                    view! { <div class="text-center py-4 pt-10">"Loading site statistics..."</div> }
                }>
                    {move || {
                        statistics_most_active_players_by_period
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(stats) => {
                                        let players: Vec<_> = stats
                                            .into_iter()
                                            .filter(|s| s.speed == selected_speed.get())
                                            .collect();
                                        if players.is_empty() {
                                            view! {
                                                <div class="text-center py-4">
                                                    <h1 class="text-xl font-bold text-center">
                                                        "Most active players"
                                                    </h1>
                                                    <h2 class="text-l text-center">"No data available"</h2>
                                                </div>
                                            }
                                                .into_any()
                                        } else {
                                            let most_active_players: Vec<TableRow> = players
                                                .iter()
                                                .map(|s| TableRow {
                                                    label: TableRowLabel::User(s.user_resp.clone()),
                                                    values: vec![s.num_games],
                                                    additional_values: None,
                                                })
                                                .collect();

                                            view! {
                                                <div class="space-y-4 text-center">
                                                    <h1 class="text-xl font-bold text-center">
                                                        "Most active players"
                                                    </h1>
                                                    <div class="grid md:grid-cols-1">
                                                        <StatTableCardGeneric
                                                            headers=vec!["Player", "Number of games"]
                                                            rows=most_active_players
                                                        />
                                                    </div>
                                                </div>
                                            }
                                                .into_any()
                                        }
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
            <section class=SECTION_STYLE>
                <Suspense fallback=move || {
                    view! { <p>"Loading..."</p> }
                }>
                    {move || {
                        rating_buckets
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(stats) => {
                                        let filtered_stats = Signal::derive(move || {
                                            stats
                                                .iter()
                                                .filter(|s| s.speed == selected_speed.get())
                                                .cloned()
                                                .collect::<Vec<_>>()
                                        });

                                        view! {
                                            <h1 class="text-xl font-bold text-center">
                                                "Player ratings distribution"
                                            </h1>
                                            <h2 class="text-l mb-4 text-center text-gray-500 dark:text-gray-300">
                                                "Users with rankable ratings. Rounded down to the nearest 10. \"All speeds\" uses players' averaged ratings"
                                            </h2>
                                            <RatingBucketsChart data=filtered_stats />
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
            <section class=SECTION_STYLE>
                <Suspense fallback=move || {
                    view! { <div class="text-center py-4 pt-10">"Loading site statistics..."</div> }
                }>
                    {move || {
                        winrate_by_rating_difference
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(stats) => {
                                        let filtered_stats: Vec<_> = if selected_speed.get()
                                            == GameSpeed::AllSpeeds
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
                                            "W advantage > 300",
                                            "W advantage 200-300",
                                            "W advantage 100-200",
                                            "W advantage < 100",
                                            "Both unrated",
                                            "B advantage < 100",
                                            "B advantage 100-200",
                                            "B advantage 200-300",
                                            "B advantage > 300",
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
                                                            label: TableRowLabel::Text(bucket.to_string()),
                                                            values: vec![*white_wins, *black_wins, *draws],
                                                            additional_values: Some(
                                                                vec![white_wins_share, black_wins_share, draws_share],
                                                            ),
                                                        }
                                                    })
                                            })
                                            .collect();

                                        view! {
                                            <h1 class="text-xl font-bold text-center">
                                                "Win rates by color"
                                            </h1>
                                            <h2 class="text-l mb-4 text-center text-gray-500 dark:text-gray-300">
                                                "Rated games"
                                            </h2>
                                            <div class="space-y-4 text-center">
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
            <section class=SECTION_STYLE>
                <Suspense fallback=move || {
                    view! { <div class="text-center py-4 pt-10">"Loading site statistics..."</div> }
                }>
                    {move || {
                        first_moves_winrate
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(stats) => {
                                        let filtered_stats: Vec<_> = stats
                                            .iter()
                                            .filter(|s| s.speed == selected_speed.get())
                                            .cloned()
                                            .collect();
                                        let first_moves_winrate_rows: Vec<TableRow> = filtered_stats
                                            .iter()
                                            .map(|s| {
                                                let total_games = s.white_wins + s.black_wins + s.draws;
                                                let white_win_rate = if total_games > 0 {
                                                    (s.white_wins as f64 / total_games as f64) * 100.0
                                                } else {
                                                    0.0
                                                };
                                                let black_win_rate = if total_games > 0 {
                                                    (s.black_wins as f64 / total_games as f64) * 100.0
                                                } else {
                                                    0.0
                                                };
                                                let draw_rate = if total_games > 0 {
                                                    (s.draws as f64 / total_games as f64) * 100.0
                                                } else {
                                                    0.0
                                                };
                                                TableRow {
                                                    label: TableRowLabel::Text(s.first_moves.clone()),
                                                    values: vec![s.white_wins, s.black_wins, s.draws],
                                                    additional_values: Some(
                                                        vec![white_win_rate, black_win_rate, draw_rate],
                                                    ),
                                                }
                                            })
                                            .collect();

                                        view! {
                                            <h1 class="text-xl font-bold text-center">
                                                "Openings win rates"
                                            </h1>
                                            <h2 class="text-l mb-4 text-center text-gray-500 dark:text-gray-300">
                                                "Rated games, top 10 openings"
                                            </h2>
                                            <div class="space-y-4 text-center">
                                                <div class="grid md:grid-cols-1">
                                                    <StatTableCardGeneric
                                                        headers=vec!["Opening", "White Wins", "Black Wins", "Draws"]
                                                        rows=first_moves_winrate_rows
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
