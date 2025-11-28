use crate::components::{atoms::rating::icon_for_speed, molecules::game_row::GameRow};
use crate::functions::games::get::GetFinishedBatchFromOptions;
use chrono::{DateTime, NaiveDate, Utc};
use leptos::{html, prelude::*};
use leptos_icons::Icon;
use leptos_router::{
    hooks::{use_navigate, use_query_map},
    location::State,
    NavigateOptions,
};
use leptos_use::{
    use_element_bounding, use_infinite_scroll_with_options, UseInfiniteScrollOptions,
};
use shared_types::{
    BatchToken, FinishedGameSortKey, FinishedGamesQueryOptions, FinishedResultFilter, GameProgress,
    GameSpeed, TimeMode,
};
use std::{str::FromStr, sync::Arc};

#[derive(Debug, Clone)]
struct GameSearchViewError(Vec<String>);

impl std::fmt::Display for GameSearchViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("; "))
    }
}

impl std::error::Error for GameSearchViewError {}

fn date_to_input(date: Option<DateTime<Utc>>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn parse_date_input(input: String) -> Option<DateTime<Utc>> {
    if input.trim().is_empty() {
        return None;
    }
    let date = NaiveDate::parse_from_str(&input, "%Y-%m-%d").ok()?;
    date.and_hms_opt(0, 0, 0)
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
}

const SELECT_CLASS: &str =
    "select select-bordered w-full bg-white text-gray-900 dark:bg-gray-800 dark:text-gray-100 dark:border-gray-700";
const PRIMARY_BUTTON_CLASS: &str = "px-4 py-2 font-bold text-white rounded transition-transform duration-200 cursor-pointer bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 focus:outline-none disabled:opacity-25 disabled:cursor-not-allowed";

fn is_realtime_speed(speed: GameSpeed) -> bool {
    GameSpeed::real_time_speeds().contains(&speed)
}

fn sanitize_speeds_for_time_mode(speeds: &mut Vec<GameSpeed>, mode: Option<TimeMode>) {
    match mode {
        Some(TimeMode::RealTime) => {
            speeds.retain(|s| is_realtime_speed(*s));
            if speeds.is_empty() {
                *speeds = GameSpeed::real_time_speeds();
            }
        }
        Some(TimeMode::Correspondence) => *speeds = vec![GameSpeed::Correspondence],
        Some(TimeMode::Untimed) => *speeds = vec![GameSpeed::Untimed],
        None => {}
    }

    speeds.sort();
    speeds.dedup();
}

fn speed_disabled_for_mode(mode: Option<TimeMode>, speed: GameSpeed) -> bool {
    match mode {
        Some(TimeMode::RealTime) => !is_realtime_speed(speed),
        Some(TimeMode::Correspondence) | Some(TimeMode::Untimed) => true,
        None => false,
    }
}

fn enforce_rating_rules(options: &mut FinishedGamesQueryOptions) {
    let rated_forbidden =
        options.time_mode == Some(TimeMode::Untimed) || options.expansions == Some(false);
    if rated_forbidden {
        options.rated = Some(false);
    } else if options.rated == Some(true) && options.speeds.contains(&GameSpeed::Untimed) {
        options.rated = None;
    }
    if options.rated != Some(true) {
        options.rating_min = None;
        options.rating_max = None;
    }
}

#[component]
pub fn GameSearch() -> impl IntoView {
    let draft_options = RwSignal::new(FinishedGamesQueryOptions::default());

    let games = RwSignal::new(Vec::new());
    let next_batch = RwSignal::new(None::<BatchToken>);
    let has_more = StoredValue::new(true);
    let total = RwSignal::new(None::<i64>);
    let errors = RwSignal::new(Vec::<String>::new());
    let applied_options = RwSignal::new(FinishedGamesQueryOptions::default());
    let next_batch_action = ServerAction::<GetFinishedBatchFromOptions>::new();
    let is_first_batch = StoredValue::new(true);
    let has_searched = RwSignal::new(false);

    let scroll_ref = NodeRef::<html::Div>::new();
    let bounding = use_element_bounding(scroll_ref);
    let infinite_scroll_batch_size = Signal::derive(move || {
        let width = bounding.width.get();
        if width < 640.0 {
            3
        } else if width < 1024.0 {
            4
        } else {
            6
        }
    });

    let is_loading = next_batch_action.pending();

    let dispatch_batch: Arc<dyn Fn(usize, bool) + Send + Sync> = {
        let options = applied_options;
        let action = next_batch_action;
        Arc::new(move |batch_size: usize, reset_lists: bool| {
            if action.pending().get_untracked() {
                return;
            }
            let mut opts = options.get_untracked();
            opts.batch_size = batch_size;
            opts.batch_token = if reset_lists {
                None
            } else {
                next_batch.get_untracked()
            };
            is_first_batch.set_value(reset_lists);

            if reset_lists {
                games.set(Vec::new());
                has_more.set_value(true);
                next_batch.set(None);
                total.set(None);
                errors.set(Vec::new());
            }

            options.set(opts.clone());
            action.dispatch(GetFinishedBatchFromOptions { options: opts });
        })
    };

    Effect::watch(
        next_batch_action.version(),
        move |_, _, _| {
            if let Some(result) = next_batch_action.value().get_untracked() {
                match result {
                    Ok(batch) => {
                        has_more.set_value(batch.next_batch.is_some());
                        total.set(Some(batch.total));
                        next_batch.set(batch.next_batch);
                        errors.update(|e| {
                            if !e.is_empty() {
                                e.clear();
                            }
                        });
                        games.update(|state| {
                            if is_first_batch.get_value() {
                                *state = batch.games;
                            } else {
                                state.extend(batch.games);
                            }
                        });
                    }
                    Err(err_msg) => {
                        errors.set(vec![err_msg.to_string()]);
                        has_more.set_value(false);
                    }
                }
            }
        },
        true,
    );

    let fetch_more = Arc::new({
        let dispatch_batch = Arc::clone(&dispatch_batch);
        move || {
            dispatch_batch(infinite_scroll_batch_size.get_untracked(), false);
        }
    });

    let navigate = use_navigate();
    let queries = use_query_map();
    {
        let dispatch_batch = Arc::clone(&dispatch_batch);

        Effect::watch(
            queries,
            move |_, _, _| {
                let query_string = queries
                    .get_untracked()
                    .into_iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join("&");
                if query_string.is_empty() {
                    return;
                }

                errors.set(Vec::new());
                match FinishedGamesQueryOptions::from_str(&query_string) {
                    Ok(mut opts) => {
                        opts.batch_size = infinite_scroll_batch_size.get_untracked();
                        opts.batch_token = None;
                        match opts.validate_all() {
                            Ok(valid) => {
                                draft_options.set(valid.clone());
                                applied_options.set(valid.clone());
                                has_searched.set(true);
                                dispatch_batch(valid.batch_size, true);
                            }
                            Err(errs) => {
                                errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                                games.set(Vec::new());
                                total.set(None);
                                has_more.set_value(false);
                                has_searched.set(true);
                            }
                        }
                    }
                    Err(e) => {
                        let msgs = match e {
                            shared_types::FinishedGamesQueryParseError::ValidationFailedList(
                                errs,
                            ) => errs.into_iter().map(|e| e.to_string()).collect(),
                            _ => vec![e.to_string()],
                        };
                        errors.set(msgs);
                        games.set(Vec::new());
                        total.set(None);
                        has_more.set_value(false);
                        has_searched.set(true);
                    }
                }
            },
            true,
        );
    }

    let start_search = {
        let navigate = navigate.clone();
        move |_| {
            let nav_options = NavigateOptions {
                resolve: true,
                replace: true,
                scroll: false,
                state: State::new(None),
            };
            errors.set(Vec::new());
            let mut opts = draft_options.get_untracked();
            opts.batch_size = infinite_scroll_batch_size.get_untracked();
            opts.batch_token = None;
            opts.game_progress = GameProgress::Finished;
            match opts.validate_all() {
                Ok(valid) => {
                    applied_options.set(valid.clone());
                    has_searched.set(true);
                    navigate(&format!("/game_search{valid}",), nav_options);
                }
                Err(errs) => {
                    errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                    games.set(Vec::new());
                    total.set(None);
                    has_more.set_value(false);
                    has_searched.set(false);
                }
            }
        }
    };

    let value = fetch_more.clone();
    let _ = use_infinite_scroll_with_options(
        scroll_ref,
        move |_| {
            let fetch_more = Arc::clone(&value);
            async move {
                if !has_searched() || !has_more.get_value() || is_loading.get() {
                    return;
                }
                fetch_more();
            }
        },
        UseInfiniteScrollOptions::default()
            .distance(10.0)
            .interval(300.0),
    );

    view! {
        <div
            node_ref=scroll_ref
            class="flex flex-col min-h-screen max-h-screen w-full bg-light dark:bg-gray-950 pt-20 overflow-y-auto"
        >
            <div class="flex-shrink-0 w-full">
                <div class="mx-auto max-w-5xl p-4 space-y-3">
                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                        <div class="space-y-3">
                            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                                <div class="space-y-1">
                                    <label class="block text-sm font-semibold">"Player 1"</label>
                                    <input
                                        class="input input-bordered w-full"
                                        placeholder="username"
                                        prop:value=Signal::derive({
                                            move || {
                                                draft_options
                                                    .with(|o| o.player1.clone().unwrap_or_default())
                                            }
                                        })
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            draft_options
                                                .update(|o| {
                                                    o.player1 = if val.trim().is_empty() {
                                                        None
                                                    } else {
                                                        Some(val.clone())
                                                    };
                                                });
                                        }
                                    />
                                </div>
                                <div class="space-y-1">
                                    <label class="block text-sm font-semibold">"Player 2"</label>
                                    <input
                                        class="input input-bordered w-full"
                                        placeholder="username"
                                        prop:value=Signal::derive({
                                            move || {
                                                draft_options
                                                    .with(|o| o.player2.clone().unwrap_or_default())
                                            }
                                        })
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            draft_options
                                                .update(|o| {
                                                    o.player2 = if val.trim().is_empty() {
                                                        None
                                                    } else {
                                                        Some(val.clone())
                                                    };
                                                });
                                        }
                                    />
                                </div>
                            </div>
                            <div class="flex items-center gap-3">
                                <label class="flex items-center gap-2 text-sm">
                                    <input
                                        type="checkbox"
                                        checked=Signal::derive({
                                            move || draft_options.with(|o| o.fixed_colors)
                                        })
                                        on:change=move |_| {
                                            draft_options.update(|o| o.fixed_colors = !o.fixed_colors)
                                        }
                                    />
                                    "Fixed colors (Player 1 white)"
                                </label>
                            </div>
                            <div class="space-y-1">
                                <label class="block text-sm font-semibold">"Result"</label>
                                <select
                                    class=SELECT_CLASS
                                    prop:value=Signal::derive({
                                        move || draft_options.with(|o| o.result_filter.to_string())
                                    })
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        if let Ok(next) = FinishedResultFilter::from_str(&val) {
                                            draft_options.update(|o| o.result_filter = next);
                                        }
                                    }
                                >
                                    <option value="any">"Any result"</option>
                                    <option value="white_wins">"White wins"</option>
                                    <option value="black_wins">"Black wins"</option>
                                    <option value="player1_wins">"Player 1 wins"</option>
                                    <option value="player2_wins">"Player 2 wins"</option>
                                    <option value="draw">"Draw"</option>
                                    <option value="not_draw">"Not a draw"</option>
                                </select>
                            </div>
                        </div>
                        <div class="space-y-3">
                            <div class="space-y-1">
                                <label class="block text-sm font-semibold">"Time mode"</label>
                                <select
                                    class=SELECT_CLASS
                                    prop:value=Signal::derive({
                                        move || {
                                            draft_options
                                                .with(|o| {
                                                    o.time_mode
                                                        .map(|tm| tm.to_string())
                                                        .unwrap_or_else(|| "any".to_string())
                                                })
                                        }
                                    })
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options
                                            .update(|o| {
                                                o.time_mode = if val == "any" {
                                                    None
                                                } else {
                                                    TimeMode::from_str(&val).ok()
                                                };
                                                sanitize_speeds_for_time_mode(&mut o.speeds, o.time_mode);
                                                enforce_rating_rules(o);
                                            });
                                    }
                                >
                                    <option value="any">"Any"</option>
                                    <option value="Real Time">"Real time"</option>
                                    <option value="Correspondence">"Correspondence"</option>
                                    <option value="Untimed">"Untimed"</option>
                                </select>
                            </div>
                            <div class="space-y-1">
                                <label class="block text-sm font-semibold">"Speeds"</label>
                                <div class="flex flex-wrap gap-2">
                                    {GameSpeed::all_games()
                                        .into_iter()
                                        .map(|speed| {
                                            let label = speed.to_string();
                                            let is_disabled = Signal::derive({
                                                move || {
                                                    draft_options
                                                        .with(|o| speed_disabled_for_mode(o.time_mode, speed))
                                                }
                                            });
                                            view! {
                                                <label class=move || {
                                                    let base = "flex items-center gap-2 text-sm px-3 py-1 rounded-lg border shadow-sm";
                                                    if is_disabled() {
                                                        format!(
                                                            "{base} cursor-not-allowed opacity-60 bg-gray-100 dark:bg-gray-800/60",
                                                        )
                                                    } else {
                                                        format!(
                                                            "{base} cursor-pointer bg-white hover:bg-gray-50 dark:bg-gray-900 dark:hover:bg-gray-800",
                                                        )
                                                    }
                                                }>
                                                    <input
                                                        type="checkbox"
                                                        prop:disabled=is_disabled
                                                        checked=Signal::derive({
                                                            move || draft_options.with(|o| o.speeds.contains(&speed))
                                                        })
                                                        on:change=move |_| {
                                                            draft_options
                                                                .update(|o| {
                                                                    if speed_disabled_for_mode(o.time_mode, speed) {
                                                                        return;
                                                                    }
                                                                    if let Some(pos) = o.speeds.iter().position(|v| v == &speed)
                                                                    {
                                                                        o.speeds.remove(pos);
                                                                    } else {
                                                                        o.speeds.push(speed);
                                                                        o.speeds.sort();
                                                                        o.speeds.dedup();
                                                                    }
                                                                    enforce_rating_rules(o);
                                                                });
                                                        }
                                                    />
                                                    <Icon icon=icon_for_speed(speed) attr:class="size-5" />
                                                    <span class="sr-only">{label.clone()}</span>
                                                </label>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        </div>
                        <div class="space-y-3">
                            <div class="space-y-1">
                                <label class="block text-sm font-semibold">"Rated"</label>
                                <select
                                    class=SELECT_CLASS
                                    prop:value=Signal::derive({
                                        move || match draft_options.with(|o| o.rated) {
                                            Some(true) => "true".to_string(),
                                            Some(false) => "false".to_string(),
                                            None => "any".to_string(),
                                        }
                                    })
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        let next = match val.as_str() {
                                            "true" => Some(true),
                                            "false" => Some(false),
                                            _ => None,
                                        };
                                        draft_options
                                            .update(|o| {
                                                o.rated = next;
                                                enforce_rating_rules(o);
                                            });
                                    }
                                >
                                    <option
                                        value="any"
                                        prop:disabled=Signal::derive({
                                            move || {
                                                draft_options
                                                    .with(|o| {
                                                        o.expansions == Some(false)
                                                            || o.time_mode == Some(TimeMode::Untimed)
                                                    })
                                            }
                                        })
                                    >
                                        "Any"
                                    </option>
                                    <option
                                        value="true"
                                        prop:disabled=Signal::derive({
                                            move || {
                                                draft_options
                                                    .with(|o| {
                                                        o.expansions == Some(false)
                                                            || o.time_mode == Some(TimeMode::Untimed)
                                                            || o.speeds.contains(&GameSpeed::Untimed)
                                                    })
                                            }
                                        })
                                    >
                                        "Rated"
                                    </option>
                                    <option value="false">"Casual"</option>
                                </select>
                            </div>
                            <div class="space-y-1">
                                <label class="block text-sm font-semibold">"Expansions"</label>
                                <select
                                    class=SELECT_CLASS
                                    prop:value=Signal::derive({
                                        move || match draft_options.with(|o| o.expansions) {
                                            Some(true) => "true".to_string(),
                                            Some(false) => "false".to_string(),
                                            None => "any".to_string(),
                                        }
                                    })
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options
                                            .update(|o| {
                                                o.expansions = match val.as_str() {
                                                    "true" => Some(true),
                                                    "false" => Some(false),
                                                    _ => None,
                                                };
                                                enforce_rating_rules(o);
                                            });
                                    }
                                >
                                    <option value="any">"Any"</option>
                                    <option value="true">"With expansions"</option>
                                    <option value="false">"Base only"</option>
                                </select>
                            </div>
                            <div class=move || {
                                format!(
                                    "space-y-1 {}",
                                    if draft_options.with(|o| o.rated != Some(true)) {
                                        "opacity-60"
                                    } else {
                                        ""
                                    },
                                )
                            }>
                                <label class="block text-sm font-semibold">"Rating range"</label>
                                <div class="grid grid-cols-2 gap-2">
                                    <input
                                        class=move || {
                                            let mut base = "input input-bordered w-full".to_string();
                                            if draft_options.with(|o| o.rated != Some(true)) {
                                                base.push_str(
                                                    " bg-gray-100 dark:bg-gray-800 cursor-not-allowed",
                                                );
                                            }
                                            base
                                        }
                                        placeholder="Min"
                                        prop:disabled=Signal::derive({
                                            move || !draft_options.with(|o| o.rated == Some(true))
                                        })
                                        prop:value=Signal::derive({
                                            move || {
                                                draft_options
                                                    .with(|o| {
                                                        o.rating_min.map(|v| v.to_string()).unwrap_or_default()
                                                    })
                                            }
                                        })
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            draft_options
                                                .update(|o| {
                                                    if o.rated != Some(true) {
                                                        return;
                                                    }
                                                    o.rating_min = val.trim().parse::<i32>().ok();
                                                });
                                        }
                                    />
                                    <input
                                        class=move || {
                                            let mut base = "input input-bordered w-full".to_string();
                                            if draft_options.with(|o| o.rated != Some(true)) {
                                                base.push_str(
                                                    " bg-gray-100 dark:bg-gray-800 cursor-not-allowed",
                                                );
                                            }
                                            base
                                        }
                                        placeholder="Max"
                                        prop:disabled=Signal::derive({
                                            move || !draft_options.with(|o| o.rated == Some(true))
                                        })
                                        prop:value=Signal::derive({
                                            move || {
                                                draft_options
                                                    .with(|o| {
                                                        o.rating_max.map(|v| v.to_string()).unwrap_or_default()
                                                    })
                                            }
                                        })
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            draft_options
                                                .update(|o| {
                                                    if o.rated != Some(true) {
                                                        return;
                                                    }
                                                    o.rating_max = val.trim().parse::<i32>().ok();
                                                });
                                        }
                                    />
                                </div>
                            </div>

                        </div>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 pt-2">
                        <div class="space-y-1">
                            <label class="block text-sm font-semibold">"Turn range"</label>
                            <div class="grid grid-cols-2 gap-2">
                                <input
                                    class="input input-bordered w-full"
                                    placeholder="Min"
                                    prop:value=Signal::derive({
                                        move || {
                                            draft_options
                                                .with(|o| {
                                                    o.turn_min.map(|v| v.to_string()).unwrap_or_default()
                                                })
                                        }
                                    })
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options
                                            .update(|o| {
                                                o.turn_min = val.trim().parse::<i32>().ok();
                                            });
                                    }
                                />
                                <input
                                    class="input input-bordered w-full"
                                    placeholder="Max"
                                    prop:value=Signal::derive({
                                        move || {
                                            draft_options
                                                .with(|o| {
                                                    o.turn_max.map(|v| v.to_string()).unwrap_or_default()
                                                })
                                        }
                                    })
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options
                                            .update(|o| {
                                                o.turn_max = val.trim().parse::<i32>().ok();
                                            });
                                    }
                                />
                            </div>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-semibold">"Date range"</label>
                            <div class="grid grid-cols-2 gap-2">
                                <input
                                    class="input input-bordered w-full"
                                    type="date"
                                    prop:value=Signal::derive({
                                        move || draft_options.with(|o| date_to_input(o.date_start))
                                    })
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options
                                            .update(|o| o.date_start = parse_date_input(val.clone()));
                                    }
                                />
                                <input
                                    class="input input-bordered w-full"
                                    type="date"
                                    prop:value=Signal::derive({
                                        move || draft_options.with(|o| date_to_input(o.date_end))
                                    })
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options
                                            .update(|o| o.date_end = parse_date_input(val.clone()));
                                    }
                                />
                            </div>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-semibold">"Sorting"</label>
                            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                                <select
                                    class=SELECT_CLASS
                                    prop:value=Signal::derive({
                                        move || draft_options.with(|o| o.sort.key.to_string())
                                    })
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        if let Ok(next) = FinishedGameSortKey::from_str(&val) {
                                            draft_options.update(|o| o.sort.key = next);
                                        }
                                    }
                                >
                                    <option value="Date">"Date"</option>
                                    <option value="Turns">"Turns"</option>
                                    <option value="RatingAvg">"Average rating"</option>
                                </select>
                                <select
                                    class=SELECT_CLASS
                                    prop:value=Signal::derive({
                                        move || {
                                            if draft_options.with(|o| o.sort.ascending) {
                                                "asc".to_string()
                                            } else {
                                                "desc".to_string()
                                            }
                                        }
                                    })
                                    on:change=move |ev| {
                                        draft_options
                                            .update(|o| {
                                                o.sort.ascending = event_target_value(&ev) == "asc";
                                            });
                                    }
                                >
                                    <option value="desc">"Newest / highest first"</option>
                                    <option value="asc">"Oldest / lowest first"</option>
                                </select>
                            </div>
                        </div>
                        <div class="flex items-center gap-3">
                            <label class="flex items-center gap-2 text-sm">
                                <input
                                    type="checkbox"
                                    checked=Signal::derive({
                                        move || draft_options.with(|o| o.exclude_bots)
                                    })
                                    on:change=move |_| {
                                        draft_options.update(|o| o.exclude_bots = !o.exclude_bots)
                                    }
                                />
                                "Exclude bots"
                            </label>
                        </div>
                        <div class="flex items-center gap-3">
                            <label class="flex items-center gap-2 text-sm">
                                <input
                                    type="checkbox"
                                    checked=Signal::derive({
                                        move || draft_options.with(|o| o.only_tournament)
                                    })
                                    on:change=move |_| {
                                        draft_options
                                            .update(|o| o.only_tournament = !o.only_tournament);
                                    }
                                />
                                "Only tournament games"
                            </label>
                        </div>
                    </div>

                    <div class="flex gap-2 pt-2">
                        <button class=PRIMARY_BUTTON_CLASS on:click=start_search>
                            "Search"
                        </button>
                    </div>
                </div>
            </div>

            <div class="px-4 pb-6">
                <div class="space-y-2">
                    <ErrorBoundary fallback=move |errors_signal| {
                        let messages = Signal::derive(move || {
                            errors_signal
                                .get()
                                .into_iter()
                                .map(|(_, err)| err.to_string())
                                .collect::<Vec<_>>()
                        });
                        view! {
                            <div class="flex-1 min-h-0 flex flex-col">
                                <div class="p-2 text-sm text-red-600 dark:text-red-400 space-y-1">
                                    <For each=move || messages.get() key=|msg| msg.clone() let:msg>
                                        <p>{msg}</p>
                                    </For>
                                </div>
                            </div>
                        }
                    }>
                        {move || {
                            if errors.with(|e| e.is_empty()) {
                                Ok(())
                            } else {
                                Err(GameSearchViewError(errors.get()))
                            }
                        }} <Show when=move || total.get().is_some()>
                            <div class="max-w-5xl mx-auto px-4 flex gap-1">
                                <p class="text-sm text-gray-700 dark:text-gray-300">
                                    {move || {
                                        let loaded = games.with(|g| g.len());
                                        total
                                            .get()
                                            .map(|t| {
                                                format!("{loaded} games loaded / {t} games found")
                                            })
                                            .unwrap_or_default()
                                    }}
                                </p>
                                <a href=move || {
                                    format!("game_search{}", queries.get().to_query_string())
                                }>Permalink</a>
                            </div>
                        </Show> <div class="space-y-2">
                            <Show when=move || has_searched.get()>
                                <div class="flex flex-col">
                                    <div class="min-h-0 rounded-lg sm:grid sm:grid-cols-2 sm:content-start lg:grid-cols-3">
                                        <For
                                            each=move || games.get()
                                            key=|game| game.game_id.clone()
                                            let:game
                                        >
                                            <GameRow game />
                                        </For>
                                    </div>
                                    <Show when=move || {
                                        games.with(|g| g.is_empty()) && !is_loading.get()
                                    }>
                                        <p class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                            "No games found."
                                        </p>
                                    </Show>
                                    <Show when=move || is_loading.get()>
                                        <p class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                            "Loading games..."
                                        </p>
                                    </Show>
                                </div>
                            </Show>
                        </div>
                    </ErrorBoundary>
                </div>
            </div>
        </div>
    }
}
