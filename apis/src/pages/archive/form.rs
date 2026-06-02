use crate::{
    common::UserAction,
    components::{
        atoms::{color_hex::ColorHex, rating::icon_for_speed},
        molecules::user_search::UserSearch,
    },
    i18n::*,
};
use chrono::{DateTime, NaiveDate, Utc};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::{GameSortKey, GameSpeed, GamesQueryOptions, ResultFilter};
use std::{collections::HashSet, str::FromStr};

// Shared field sizing/appearance so inputs, selects and the player-search box
// all line up (same height, padding, radius, border, focus ring).
const FIELD_CLASS: &str = "w-full min-h-10 px-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-white text-gray-900 dark:bg-gray-800 dark:text-gray-100 shadow-sm focus:outline-none focus:ring-2 focus:ring-pillbug-teal/50 focus:border-pillbug-teal";
const SELECT_CLASS: &str = FIELD_CLASS;
const INPUT_CLASS: &str = FIELD_CLASS;
const LABEL_CLASS: &str = "block text-sm font-semibold text-gray-700 dark:text-gray-200";
const FIELD_BLOCK_CLASS: &str = "space-y-1.5";
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

fn enforce_rating_rules(options: &mut GamesQueryOptions) {
    if options.rated != Some(true) {
        options.rating_min = None;
        options.rating_max = None;
    }
}

/// Highlight-on-select pill, matching the speed toggles, for boolean filters.
#[component]
fn TogglePill(checked: Signal<bool>, on_toggle: Callback<()>, children: Children) -> impl IntoView {
    view! {
        <label class=move || {
            let base = "inline-flex gap-2 justify-center items-center px-3 min-h-10 text-sm rounded-lg border-2 shadow-sm transition-colors duration-150 cursor-pointer select-none";
            if checked.get() {
                format!("{base} border-pillbug-teal bg-pillbug-teal/10 text-pillbug-teal")
            } else {
                format!(
                    "{base} border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-900 hover:bg-gray-50 dark:hover:bg-gray-800 hover:border-pillbug-teal/40",
                )
            }
        }>
            <input
                type="checkbox"
                class="sr-only"
                prop:checked=checked
                on:change=move |_| on_toggle.run(())
            />
            {children()}
        </label>
    }
}

#[component]
fn ArchivePlayerField(
    label: impl Fn() -> String + 'static + Send,
    placeholder: String,
    draft_options: RwSignal<GamesQueryOptions>,
    is_player1: bool,
) -> impl IntoView {
    let filtered = Signal::derive(move || {
        draft_options.with(|o| {
            (if is_player1 {
                o.player2.clone()
            } else {
                o.player1.clone()
            })
            .into_iter()
            .collect::<HashSet<_>>()
        })
    });
    let value = Signal::derive(move || {
        draft_options.with(|o| {
            if is_player1 {
                o.player1.clone()
            } else {
                o.player2.clone()
            }
        })
    });
    // Used for both picking a suggestion and typing free text, so clicking
    // Search applies whatever is in the field even without selecting a row.
    let set_player = Callback::new(move |opt: Option<String>| {
        draft_options.update(|o| {
            if is_player1 {
                o.player1 = opt;
            } else {
                o.player2 = opt;
            }
        });
    });
    // Player 1 plays White, player 2 plays Black when fixed colors is on.
    let color = Signal::derive(move || {
        if is_player1 {
            Color::White
        } else {
            Color::Black
        }
    });
    let fixed_colors = Signal::derive(move || draft_options.with(|o| o.fixed_colors));

    view! {
        <div class=FIELD_BLOCK_CLASS>
            <label class="flex gap-1 items-center text-sm font-semibold text-gray-700 dark:text-gray-200">
                {move || label()} <Show when=move || fixed_colors.get()>
                    <ColorHex color=color />
                </Show>
            </label>
            <UserSearch
                placeholder=placeholder
                filtered_users_signal=filtered
                value=value
                on_input=set_player
                input_class=INPUT_CLASS.to_string()
                compact=true
                actions=vec![UserAction::Select(set_player)]
            />
        </div>
    }
}

#[component]
fn ArchiveSpeedField(draft_options: RwSignal<GamesQueryOptions>) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class=FIELD_BLOCK_CLASS>
            <label class=LABEL_CLASS>{t!(i18n, archive.speeds)}</label>
            <div class="flex flex-wrap gap-2">
                {GameSpeed::all_games()
                    .into_iter()
                    .map(|speed| {
                        let label = speed.to_string();
                        let is_selected = Signal::derive(move || {
                            draft_options.with(|o| o.speeds.contains(&speed))
                        });
                        view! {
                            <label class=move || {
                                let base = "flex justify-center items-center text-sm rounded-lg border-2 shadow-sm transition-colors duration-150 size-10";
                                if is_selected() {
                                    format!(
                                        "{base} cursor-pointer border-pillbug-teal bg-pillbug-teal/10 text-pillbug-teal",
                                    )
                                } else {
                                    format!(
                                        "{base} cursor-pointer border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-900 hover:bg-gray-50 dark:hover:bg-gray-800 hover:border-pillbug-teal/40",
                                    )
                                }
                            }>
                                <input
                                    type="checkbox"
                                    class="sr-only"
                                    checked=is_selected
                                    on:change=move |_| {
                                        draft_options
                                            .update(|o| {
                                                if let Some(pos) = o.speeds.iter().position(|v| v == &speed)
                                                {
                                                    if o.speeds.len() > 1 {
                                                        o.speeds.remove(pos);
                                                    }
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
    }
}

#[component]
fn ArchiveRatedExpansions(draft_options: RwSignal<GamesQueryOptions>) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="grid grid-cols-2 gap-3">
            <div class=FIELD_BLOCK_CLASS>
                <label class=LABEL_CLASS>{t!(i18n, archive.rated)}</label>
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
                        draft_options
                            .update(|o| {
                                o.rated = next;
                                enforce_rating_rules(o);
                            });
                    }
                >
                    <option value="any">{t!(i18n, archive.any)}</option>
                    <option value="true">{t!(i18n, archive.rated)}</option>
                    <option value="false">{t!(i18n, archive.casual)}</option>
                </select>
            </div>
            <div class=FIELD_BLOCK_CLASS>
                <label class=LABEL_CLASS>{t!(i18n, archive.expansions)}</label>
                <select
                    class=SELECT_CLASS
                    prop:value=Signal::derive(move || match draft_options.with(|o| o.expansions) {
                        Some(true) => "true".to_string(),
                        Some(false) => "false".to_string(),
                        None => "any".to_string(),
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
                    <option value="any">{t!(i18n, archive.any)}</option>
                    <option value="true">{t!(i18n, archive.with_expansions)}</option>
                    <option value="false">{t!(i18n, archive.base_only)}</option>
                </select>
            </div>
        </div>
    }
}

#[component]
fn ArchiveAdvancedFilters(
    draft_options: RwSignal<GamesQueryOptions>,
    on_search: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="space-y-3">
            <div class=move || {
                format!(
                    "{FIELD_BLOCK_CLASS} {}",
                    if draft_options.with(|o| o.rated != Some(true)) { "opacity-60" } else { "" },
                )
            }>
                <label class=LABEL_CLASS>{t!(i18n, archive.rating_range)}</label>
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
                        prop:disabled=Signal::derive(move || {
                            !draft_options.with(|o| o.rated == Some(true))
                        })
                        prop:value=Signal::derive(move || {
                            draft_options
                                .with(|o| o.rating_min.map(|v| v.to_string()).unwrap_or_default())
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
                            let mut base = INPUT_CLASS.to_string();
                            if draft_options.with(|o| o.rated != Some(true)) {
                                base.push_str(" bg-gray-100 dark:bg-gray-800 cursor-not-allowed");
                            }
                            base
                        }
                        placeholder=move || t_string!(i18n, archive.max)
                        prop:disabled=Signal::derive(move || {
                            !draft_options.with(|o| o.rated == Some(true))
                        })
                        prop:value=Signal::derive(move || {
                            draft_options
                                .with(|o| o.rating_max.map(|v| v.to_string()).unwrap_or_default())
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
            <div class=FIELD_BLOCK_CLASS>
                <label class=LABEL_CLASS>{t!(i18n, archive.turn_range)}</label>
                <div class="grid grid-cols-2 gap-2">
                    <input
                        class=INPUT_CLASS
                        placeholder=move || t_string!(i18n, archive.min)
                        prop:value=Signal::derive(move || {
                            draft_options
                                .with(|o| o.turn_min.map(|v| v.to_string()).unwrap_or_default())
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
                            draft_options
                                .with(|o| o.turn_max.map(|v| v.to_string()).unwrap_or_default())
                        })
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            draft_options.update(|o| o.turn_max = val.trim().parse::<i32>().ok());
                        }
                    />
                </div>
            </div>
            <div class=FIELD_BLOCK_CLASS>
                <label class=LABEL_CLASS>{t!(i18n, archive.date_range)}</label>
                <div class="grid grid-cols-2 gap-2">
                    <input
                        class=INPUT_CLASS
                        type="date"
                        prop:value=Signal::derive(move || {
                            draft_options.with(|o| date_to_input(o.date_start))
                        })
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            draft_options.update(|o| o.date_start = parse_date_input(val.clone()));
                        }
                    />
                    <input
                        class=INPUT_CLASS
                        type="date"
                        prop:value=Signal::derive(move || {
                            draft_options.with(|o| date_to_input(o.date_end))
                        })
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            draft_options.update(|o| o.date_end = parse_date_input(val.clone()));
                        }
                    />
                </div>
            </div>
            <div class=FIELD_BLOCK_CLASS>
                <label class=LABEL_CLASS>{t!(i18n, archive.sorting)}</label>
                <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
                    <select
                        class=SELECT_CLASS
                        prop:value=Signal::derive(move || {
                            draft_options.with(|o| o.sort.key.to_string())
                        })
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(next) = GameSortKey::from_str(&val) {
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
                            draft_options
                                .update(|o| {
                                    o.sort.ascending = event_target_value(&ev) == "asc";
                                });
                        }
                    >
                        <option value="desc">{t!(i18n, archive.sort_desc)}</option>
                        <option value="asc">{t!(i18n, archive.sort_asc)}</option>
                    </select>
                </div>
            </div>
            <div class="flex flex-wrap gap-2 items-center">
                <TogglePill
                    checked=Signal::derive(move || draft_options.with(|o| o.exclude_bots))
                    on_toggle=Callback::new(move |_| {
                        draft_options.update(|o| o.exclude_bots = !o.exclude_bots)
                    })
                >
                    {t!(i18n, archive.exclude_bots)}
                </TogglePill>
                <TogglePill
                    checked=Signal::derive(move || draft_options.with(|o| o.only_tournament))
                    on_toggle=Callback::new(move |_| {
                        draft_options.update(|o| o.only_tournament = !o.only_tournament)
                    })
                >
                    {t!(i18n, archive.only_tournament)}
                </TogglePill>
                // Shown only in the two-column (md+) layout; the one-column
                // layout keeps the full-width Search button at the bottom.
                <button
                    type="button"
                    class=format!(
                        "{PRIMARY_BUTTON_CLASS} hidden items-center md:inline-flex md:ml-auto",
                    )
                    on:click=move |_| on_search.run(())
                >
                    {t!(i18n, archive.search)}
                </button>
            </div>
        </div>
    }
}

/// Active "search by board position" chip. Only shown when a position hash is
/// set (it arrives via the `/archive?hash=...` link from the analysis page or a
/// permalink). The × clears the filter.
#[component]
fn PositionFilterChip(draft_options: RwSignal<GamesQueryOptions>) -> impl IntoView {
    let i18n = use_i18n();
    let active = Signal::derive(move || draft_options.with(|o| o.position_hash.is_some()));

    view! {
        <Show when=move || active.get()>
            <div class=FIELD_BLOCK_CLASS>
                <label class=LABEL_CLASS>{t!(i18n, archive.position_filter)}</label>
                <div class="inline-flex gap-2 items-center px-3 text-sm rounded-lg border-2 shadow-sm min-h-10 border-pillbug-teal bg-pillbug-teal/10 text-pillbug-teal">
                    <span>{t!(i18n, archive.position_filter_active)}</span>
                    <button
                        type="button"
                        class="inline-flex justify-center items-center ml-1 text-base leading-none rounded-full transition-colors cursor-pointer size-5 hover:bg-pillbug-teal/20"
                        aria-label=move || t_string!(i18n, archive.clear_position).to_string()
                        on:click=move |_| draft_options.update(|o| o.position_hash = None)
                    >
                        "✕"
                    </button>
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn ArchiveSearchForm(
    draft_options: RwSignal<GamesQueryOptions>,
    on_search: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    // Read once, untracked: the placeholder is non-reactive and reading the
    // locale signal in the component body would warn otherwise.
    let username_placeholder =
        Signal::derive(move || t_string!(i18n, archive.username_placeholder).to_string())
            .get_untracked();

    view! {
        <div class="flex-shrink-0 w-full border-b border-gray-200 shadow-sm dark:border-gray-800 bg-white/80 backdrop-blur-sm dark:bg-gray-900/80">
            <div class="p-4 mx-auto space-y-4 max-w-5xl sm:p-6">
                <div class="grid grid-cols-1 gap-4 items-start sm:gap-6 md:grid-cols-2">
                    <div class="space-y-3">
                        <PositionFilterChip draft_options=draft_options />
                        <div class="space-y-2">
                            <div class="grid grid-cols-1 gap-y-2 gap-x-3 sm:grid-cols-2">
                                <ArchivePlayerField
                                    label=move || t_string!(i18n, archive.player1).to_string()
                                    placeholder=username_placeholder.clone()
                                    draft_options=draft_options
                                    is_player1=true
                                />
                                <ArchivePlayerField
                                    label=move || t_string!(i18n, archive.player2).to_string()
                                    placeholder=username_placeholder.clone()
                                    draft_options=draft_options
                                    is_player1=false
                                />
                            </div>
                            <div class="flex">
                                <TogglePill
                                    checked=Signal::derive(move || {
                                        draft_options.with(|o| o.fixed_colors)
                                    })
                                    on_toggle=Callback::new(move |_| {
                                        draft_options.update(|o| o.fixed_colors = !o.fixed_colors)
                                    })
                                >
                                    {t!(i18n, archive.fixed_colors)}
                                </TogglePill>
                            </div>
                        </div>
                        <ArchiveSpeedField draft_options=draft_options />
                        <div class=FIELD_BLOCK_CLASS>
                            <label class=LABEL_CLASS>{t!(i18n, archive.result)}</label>
                            <select
                                class=SELECT_CLASS
                                prop:value=Signal::derive(move || {
                                    draft_options.with(|o| o.result_filter.to_string())
                                })
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    if let Ok(next) = ResultFilter::from_str(&val) {
                                        draft_options.update(|o| o.result_filter = next);
                                    }
                                }
                            >
                                <option value="any">{t!(i18n, archive.result_any)}</option>
                                <option value="white_wins">
                                    {t!(i18n, archive.result_white_wins)}
                                </option>
                                <option value="black_wins">
                                    {t!(i18n, archive.result_black_wins)}
                                </option>
                                <option value="player1_wins">
                                    {t!(i18n, archive.result_player1_wins)}
                                </option>
                                <option value="player2_wins">
                                    {t!(i18n, archive.result_player2_wins)}
                                </option>
                                <option value="draw">{t!(i18n, archive.result_draw)}</option>
                                <option value="not_draw">
                                    {t!(i18n, archive.result_not_draw)}
                                </option>
                            </select>
                        </div>
                        <ArchiveRatedExpansions draft_options=draft_options />
                    </div>

                    <details class="rounded-xl border-2 border-gray-200 shadow-sm md:hidden dark:border-gray-700 group bg-gray-50/80 dark:bg-gray-800/50">
                        <summary class="flex gap-2 items-center py-3 px-4 text-sm font-semibold list-none text-gray-700 rounded-t-xl transition-colors cursor-pointer dark:text-gray-200 hover:bg-gray-100 [&::-webkit-details-marker]:hidden dark:hover:bg-gray-700/50">
                            <Icon
                                icon=icondata_lu::LuChevronDown
                                attr:class="size-4 transition-transform group-open:rotate-180 text-pillbug-teal"
                            />
                            {t!(i18n, archive.more_filters)}
                        </summary>
                        <div class="p-4 space-y-3 border-t border-gray-200 dark:border-gray-700">
                            <ArchiveAdvancedFilters
                                draft_options=draft_options
                                on_search=on_search
                            />
                        </div>
                    </details>

                    <div class="hidden md:block">
                        <ArchiveAdvancedFilters draft_options=draft_options on_search=on_search />
                    </div>
                </div>

                <div class="flex pt-2 sm:justify-end sm:pt-4 md:hidden">
                    <button
                        type="button"
                        class=format!("{PRIMARY_BUTTON_CLASS} w-full sm:w-auto")
                        on:click=move |_| on_search.run(())
                    >
                        {t!(i18n, archive.search)}
                    </button>
                </div>
            </div>
        </div>
    }
}
