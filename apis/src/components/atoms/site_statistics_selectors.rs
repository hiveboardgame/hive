use crate::components::atoms::rating::icon_for_speed;
use crate::components::layouts::base_layout::OrientationSignal;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{GameSpeed, GameTypeFilter, TimePeriod};

const BASE_SELECTOR_STYLE: &str = "no-link-style py-1 px-2 text-sm font-semibold rounded-lg border-2 transition-all duration-200 transform hover:scale-[1.02] cursor-pointer shadow-sm hover:shadow-md";
const ACTIVE_SELECTOR_STYLE: &str =
    "bg-pillbug-teal border-pillbug-teal text-white hover:bg-pillbug-teal/90";
const INACTIVE_SELECTOR_STYLE: &str = "bg-gray-50 border-gray-200 text-gray-700 hover:bg-gray-100 hover:border-gray-300 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:border-gray-500";

#[component]
pub fn PeriodSelector(selected_period: RwSignal<TimePeriod>) -> impl IntoView {
    let periods = TimePeriod::all();

    view! {
        <div class="flex gap-2 mb-4 flex-wrap justify-center">
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
                                selected_period.set(period)
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
pub fn GamesTypesSelector(included_game_types: RwSignal<GameTypeFilter>) -> impl IntoView {
    let game_types = GameTypeFilter::all();

    view! {
        <div class="flex gap-2 mb-4 flex-wrap justify-center">
            {game_types
                .into_iter()
                .map(|gt| {
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if included_game_types.get() == gt {
                                    format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                } else {
                                    format!("{} {}", BASE_SELECTOR_STYLE, INACTIVE_SELECTOR_STYLE)
                                }
                            }
                            on:click=move |e| {
                                e.prevent_default();
                                included_game_types.set(gt)
                            }
                        >
                            {gt.to_string()}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub fn SpeedSelector(speeds: Vec<GameSpeed>, selected_speed: RwSignal<GameSpeed>) -> impl IntoView {
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    view! {
        <div class="flex gap-2 mb-4 flex-wrap justify-center">
            {speeds
                .iter()
                .map(|speed| {
                    let speed_value = *speed;
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if selected_speed.get() == speed_value {
                                    format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                } else {
                                    format!("{} {}", BASE_SELECTOR_STYLE, INACTIVE_SELECTOR_STYLE)
                                }
                            }
                            on:click=move |e| {
                                e.prevent_default();
                                selected_speed.set(speed_value);
                            }
                        >
                            <div class="flex flex-row gap-1 items-center">
                                <Icon icon=icon_for_speed(speed_value) attr:class="size-4" />
                                {move || {
                                    if !vertical.get() {
                                        speed_value.to_string()
                                    } else {
                                        "".to_string()
                                    }
                                }}
                            </div>
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub fn IncludeBotGamesSelector(include_bots: RwSignal<bool>) -> impl IntoView {
    let include_bots_vec = vec![(true, "Include Bots"), (false, "Exclude Bots")];

    view! {
        <div class="flex gap-2 mb-4 flex-wrap justify-center">
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
                                include_bots.set(value)
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
