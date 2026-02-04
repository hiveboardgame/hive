use crate::common::UserAction;
use crate::components::{atoms::rating::icon_for_speed, molecules::user_search::UserSearch};
use crate::i18n::*;
use chrono::{DateTime, NaiveDate, Utc};
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::{
    FinishedGameSortKey, FinishedGamesQueryOptions, FinishedResultFilter, GameSpeed, TimeMode,
};
use std::collections::HashSet;
use std::str::FromStr;

const SELECT_CLASS: &str =
    "select select-bordered w-full min-h-10 rounded-lg border border-gray-300 dark:border-gray-600 bg-white text-gray-900 dark:bg-gray-800 dark:text-gray-100 shadow-sm focus:ring-2 focus:ring-pillbug-teal/50 focus:border-pillbug-teal";
const INPUT_CLASS: &str =
    "input input-bordered w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white text-gray-900 dark:bg-gray-800 dark:text-gray-100 shadow-sm focus:ring-2 focus:ring-pillbug-teal/50";
const PRIMARY_BUTTON_CLASS: &str =
    "px-6 py-2.5 font-bold text-white rounded-lg shadow-md transition-all duration-200 cursor-pointer bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal hover:shadow-lg active:scale-[0.98] focus:outline-none focus:ring-2 focus:ring-pillbug-teal/50 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:shadow-md";

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

fn is_realtime_speed(speed: GameSpeed) -> bool {
    GameSpeed::real_time_speeds().contains(&speed)
}

fn sanitize_speeds_for_time_mode(speeds: &mut Vec<GameSpeed>, mode: Option<TimeMode>) {
    let new_speeds = match mode {
        Some(TimeMode::RealTime) => {
            let mut filtered: Vec<_> = speeds.iter().copied().filter(|s| is_realtime_speed(*s)).collect();
            if filtered.is_empty() {
                GameSpeed::real_time_speeds()
            } else {
                filtered.sort();
                filtered.dedup();
                filtered
            }
        }
        Some(TimeMode::Correspondence) => vec![GameSpeed::Correspondence],
        Some(TimeMode::Untimed) => vec![GameSpeed::Untimed],
        None => GameSpeed::all_games(),
    };
    *speeds = new_speeds;
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
fn ArchivePlayerField(
    label: impl Fn() -> String + 'static + Send,
    placeholder: String,
    draft_options: RwSignal<FinishedGamesQueryOptions>,
    is_player1: bool,
) -> impl IntoView {
    let filtered = Signal::derive(move || {
        draft_options.with(|o| {
            (if is_player1 { o.player2.clone() } else { o.player1.clone() })
                .into_iter()
                .collect::<HashSet<_>>()
        })
    });
    let value = Signal::derive(move || {
        draft_options.with(|o| if is_player1 { o.player1.clone() } else { o.player2.clone() })
    });
    let on_select = Callback::new(move |opt: Option<String>| {
        draft_options.update(|o| {
            if is_player1 {
                o.player1 = opt;
            } else {
                o.player2 = opt;
            }
        });
    });

    view! {
        <div class="space-y-1">
            <label class="block text-sm font-semibold">{move || label()}</label>
            <UserSearch
                placeholder=placeholder
                filtered_users_signal=filtered
                value=value
                actions=vec![UserAction::Select(on_select)]
            />
        </div>
    }
}

#[component]
fn ArchiveAdvancedFilters(draft_options: RwSignal<FinishedGamesQueryOptions>) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="space-y-3">
            <div class="space-y-1">
                <label class="block text-sm font-semibold">{t!(i18n, archive.time_mode)}</label>
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || {
                                    draft_options
                                        .with(|o| {
                                            o.time_mode
                                                .map(|tm| tm.to_string())
                                                .unwrap_or_else(|| "any".to_string())
                                        })
                                })
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    draft_options.update(|o| {
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
                                <option value="any">{t!(i18n, archive.any)}</option>
                                <option value="Real Time">{t!(i18n, archive.real_time)}</option>
                                <option value="Correspondence">{t!(i18n, archive.correspondence)}</option>
                                <option value="Untimed">{t!(i18n, archive.untimed)}</option>
                            </select>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-semibold">{t!(i18n, archive.speeds)}</label>
                            <div class="flex flex-wrap gap-2">
                                {GameSpeed::all_games()
                                    .into_iter()
                                    .map(|speed| {
                                        let label = speed.to_string();
                                        let is_disabled = Signal::derive(move || {
                                            draft_options.with(|o| speed_disabled_for_mode(o.time_mode, speed))
                                        });
                                        view! {
                                            <label class=move || {
                                                let base = "flex items-center gap-2 text-sm px-3 py-2 rounded-lg border-2 border-gray-200 dark:border-gray-600 transition-all duration-150";
                                                if is_disabled() {
                                                    format!("{base} cursor-not-allowed opacity-50 bg-gray-100 dark:bg-gray-800/60")
                                                } else {
                                                    format!("{base} cursor-pointer bg-white hover:bg-gray-50 hover:border-pillbug-teal/40 dark:bg-gray-900 dark:hover:bg-gray-800 dark:hover:border-pillbug-teal/40 shadow-sm")
                                                }
                                            }>
                                                <input
                                                    type="checkbox"
                                                    prop:disabled=is_disabled
                                                    checked=Signal::derive(move || draft_options.with(|o| o.speeds.contains(&speed)))
                                                    on:change=move |_| {
                                                        draft_options.update(|o| {
                                                            if speed_disabled_for_mode(o.time_mode, speed) {
                                                                return;
                                                            }
                                                            if let Some(pos) = o.speeds.iter().position(|v| v == &speed) {
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
                        <div class="space-y-3">
                            <div class="space-y-1">
                                <label class="block text-sm font-semibold">{t!(i18n, archive.rated)}</label>
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || match draft_options.with(|o| o.rated) {
                                    Some(true) => "true".to_string(),
                                    Some(false) => "false".to_string(),
                                    None => "any".to_string(),
                                })
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    let next = match val.as_str() {
                                        "true" => Some(true),
                                        "false" => Some(false),
                                        _ => None,
                                    };
                                    draft_options.update(|o| {
                                        o.rated = next;
                                        enforce_rating_rules(o);
                                    });
                                }
                            >
                                <option
                                    value="any"
                                    prop:disabled=Signal::derive(move || {
                                        draft_options.with(|o| {
                                            o.expansions == Some(false)
                                                || o.time_mode == Some(TimeMode::Untimed)
                                        })
                                    })
                                >
                                    {t!(i18n, archive.any)}
                                </option>
                                <option
                                    value="true"
                                    prop:disabled=Signal::derive(move || {
                                        draft_options.with(|o| {
                                            o.expansions == Some(false)
                                                || o.time_mode == Some(TimeMode::Untimed)
                                                || o.speeds.contains(&GameSpeed::Untimed)
                                        })
                                    })
                                >
                                    {t!(i18n, archive.rated)}
                                </option>
                                <option value="false">{t!(i18n, archive.casual)}</option>
                            </select>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-semibold">{t!(i18n, archive.expansions)}</label>
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || match draft_options.with(|o| o.expansions) {
                                    Some(true) => "true".to_string(),
                                    Some(false) => "false".to_string(),
                                    None => "any".to_string(),
                                })
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    draft_options.update(|o| {
                                        o.expansions = match val.as_str() {
                                            "true" => Some(true),
                                            "false" => Some(false),
                                            _ => None,
                                        };
                                        enforce_rating_rules(o);
                                    });
                                }
                            >
                                <option value="any">{t!(i18n, archive.any)}</option>
                                <option value="true">{t!(i18n, archive.with_expansions)}</option>
                                <option value="false">{t!(i18n, archive.base_only)}</option>
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
                            <label class="block text-sm font-semibold">{t!(i18n, archive.rating_range)}</label>
                            <div class="grid grid-cols-2 gap-2">
                            <input
                                class=move || {
                                    let mut base = INPUT_CLASS.to_string();
                                    if draft_options.with(|o| o.rated != Some(true)) {
                                        base.push_str(" bg-gray-100 dark:bg-gray-800 cursor-not-allowed");
                                    }
                                    base
                                }
                                    placeholder=move || t_string!(i18n, archive.min)
                                    prop:disabled=Signal::derive(move || !draft_options.with(|o| o.rated == Some(true)))
                                    prop:value=Signal::derive(move || {
                                        draft_options.with(|o| o.rating_min.map(|v| v.to_string()).unwrap_or_default())
                                    })
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options.update(|o| {
                                            if o.rated != Some(true) {
                                                return;
                                            }
                                            o.rating_min = val.trim().parse::<i32>().ok();
                                        });
                                    }
                                />
                            <input
                                class=move || {
                                    let mut base = INPUT_CLASS.to_string();
                                    if draft_options.with(|o| o.rated != Some(true)) {
                                        base.push_str(" bg-gray-100 dark:bg-gray-800 cursor-not-allowed");
                                    }
                                    base
                                }
                                    placeholder=move || t_string!(i18n, archive.max)
                                    prop:disabled=Signal::derive(move || !draft_options.with(|o| o.rated == Some(true)))
                                    prop:value=Signal::derive(move || {
                                        draft_options.with(|o| o.rating_max.map(|v| v.to_string()).unwrap_or_default())
                                    })
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        draft_options.update(|o| {
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

                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 pt-2">
                    <div class="space-y-1">
                        <label class="block text-sm font-semibold">{t!(i18n, archive.turn_range)}</label>
                        <div class="grid grid-cols-2 gap-2">
                            <input
                                class=INPUT_CLASS
                                placeholder=move || t_string!(i18n, archive.min)
                                prop:value=Signal::derive(move || {
                                    draft_options.with(|o| o.turn_min.map(|v| v.to_string()).unwrap_or_default())
                                })
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    draft_options.update(|o| o.turn_min = val.trim().parse::<i32>().ok());
                                }
                            />
                            <input
                                class=INPUT_CLASS
                                placeholder=move || t_string!(i18n, archive.max)
                                prop:value=Signal::derive(move || {
                                    draft_options.with(|o| o.turn_max.map(|v| v.to_string()).unwrap_or_default())
                                })
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    draft_options.update(|o| o.turn_max = val.trim().parse::<i32>().ok());
                                }
                            />
                        </div>
                    </div>
                    <div class="space-y-1">
                        <label class="block text-sm font-semibold">{t!(i18n, archive.date_range)}</label>
                        <div class="grid grid-cols-2 gap-2">
                            <input
                                class=INPUT_CLASS
                                type="date"
                                prop:value=Signal::derive(move || draft_options.with(|o| date_to_input(o.date_start)))
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    draft_options.update(|o| o.date_start = parse_date_input(val.clone()));
                                }
                            />
                            <input
                                class=INPUT_CLASS
                                type="date"
                                prop:value=Signal::derive(move || draft_options.with(|o| date_to_input(o.date_end)))
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    draft_options.update(|o| o.date_end = parse_date_input(val.clone()));
                                }
                            />
                        </div>
                    </div>
                    <div class="space-y-1">
                        <label class="block text-sm font-semibold">{t!(i18n, archive.sorting)}</label>
                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || draft_options.with(|o| o.sort.key.to_string()))
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    if let Ok(next) = FinishedGameSortKey::from_str(&val) {
                                        draft_options.update(|o| o.sort.key = next);
                                    }
                                }
                            >
                                <option value="Date">{t!(i18n, archive.sort_date)}</option>
                                <option value="Turns">{t!(i18n, archive.sort_turns)}</option>
                                <option value="RatingAvg">{t!(i18n, archive.sort_rating_avg)}</option>
                            </select>
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || {
                                    if draft_options.with(|o| o.sort.ascending) {
                                        "asc".to_string()
                                    } else {
                                        "desc".to_string()
                                    }
                                })
                                on:change=move |ev| {
                                    draft_options.update(|o| {
                                        o.sort.ascending = event_target_value(&ev) == "asc";
                                    });
                                }
                            >
                                <option value="desc">{t!(i18n, archive.sort_desc)}</option>
                                <option value="asc">{t!(i18n, archive.sort_asc)}</option>
                            </select>
                        </div>
                    </div>
                    <div class="flex items-center gap-3">
                        <label class="flex items-center gap-2 text-sm">
                            <input
                                type="checkbox"
                                checked=Signal::derive(move || draft_options.with(|o| o.exclude_bots))
                                on:change=move |_| draft_options.update(|o| o.exclude_bots = !o.exclude_bots)
                            />
                            {t!(i18n, archive.exclude_bots)}
                        </label>
                    </div>
                    <div class="flex items-center gap-3">
                        <label class="flex items-center gap-2 text-sm">
                            <input
                                type="checkbox"
                                checked=Signal::derive(move || draft_options.with(|o| o.only_tournament))
                                on:change=move |_| draft_options.update(|o| o.only_tournament = !o.only_tournament)
                            />
                            {t!(i18n, archive.only_tournament)}
                        </label>
                    </div>
                </div>
        </div>
    }
}

#[component]
pub fn ArchiveSearchForm(
    draft_options: RwSignal<FinishedGamesQueryOptions>,
    on_search: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let username_placeholder = move || t_string!(i18n, archive.username_placeholder).to_string();

    view! {
        <div class="flex-shrink-0 w-full border-b border-gray-200 dark:border-gray-800 bg-white/80 dark:bg-gray-900/80 backdrop-blur-sm shadow-sm">
            <div class="mx-auto max-w-5xl p-4 sm:p-6 space-y-4">
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-6">
                    <div class="space-y-3">
                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                            <ArchivePlayerField
                                label=move || t_string!(i18n, archive.player1).to_string()
                                placeholder=username_placeholder()
                                draft_options=draft_options
                                is_player1=true
                            />
                            <ArchivePlayerField
                                label=move || t_string!(i18n, archive.player2).to_string()
                                placeholder=username_placeholder()
                                draft_options=draft_options
                                is_player1=false
                            />
                        </div>
                        <div class="flex items-center gap-3">
                            <label class="flex items-center gap-2 text-sm">
                                <input
                                    type="checkbox"
                                    checked=Signal::derive(move || draft_options.with(|o| o.fixed_colors))
                                    on:change=move |_| draft_options.update(|o| o.fixed_colors = !o.fixed_colors)
                                />
                                {t!(i18n, archive.fixed_colors)}
                            </label>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-semibold">{t!(i18n, archive.result)}</label>
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || draft_options.with(|o| o.result_filter.to_string()))
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    if let Ok(next) = FinishedResultFilter::from_str(&val) {
                                        draft_options.update(|o| o.result_filter = next);
                                    }
                                }
                            >
                                <option value="any">{t!(i18n, archive.result_any)}</option>
                                <option value="white_wins">{t!(i18n, archive.result_white_wins)}</option>
                                <option value="black_wins">{t!(i18n, archive.result_black_wins)}</option>
                                <option value="player1_wins">{t!(i18n, archive.result_player1_wins)}</option>
                                <option value="player2_wins">{t!(i18n, archive.result_player2_wins)}</option>
                                <option value="draw">{t!(i18n, archive.result_draw)}</option>
                                <option value="not_draw">{t!(i18n, archive.result_not_draw)}</option>
                            </select>
                        </div>
                    </div>

                    <details class="group md:hidden mt-2 rounded-xl border-2 border-gray-200 dark:border-gray-700 bg-gray-50/80 dark:bg-gray-800/50 shadow-sm">
                        <summary class="cursor-pointer px-4 py-3 text-sm font-semibold text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/50 rounded-t-xl list-none flex items-center gap-2 [&::-webkit-details-marker]:hidden transition-colors">
                            <Icon icon=icondata_lu::LuChevronDown attr:class="size-4 transition-transform group-open:rotate-180 text-pillbug-teal" />
                            {t!(i18n, archive.more_filters)}
                        </summary>
                        <div class="p-4 pt-0 space-y-3 border-t border-gray-200 dark:border-gray-700">
                            <ArchiveAdvancedFilters draft_options=draft_options />
                        </div>
                    </details>

                    <div class="hidden md:block md:col-span-2">
                        <ArchiveAdvancedFilters draft_options=draft_options />
                    </div>
                </div>

                <div class="flex flex-wrap gap-3 pt-2 sm:pt-4">
                    <button
                        type="button"
                        class=PRIMARY_BUTTON_CLASS
                        on:click=move |_| on_search.run(())
                    >
                        {t!(i18n, archive.search)}
                    </button>
                </div>
            </div>
        </div>
    }
}
