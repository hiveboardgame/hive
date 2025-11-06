use leptos::prelude::*;
use shared_types::SiteStatisticsTimePeriod;

const BASE_SELECTOR_STYLE: &str = "no-link-style py-1 px-2 text-sm font-semibold rounded-lg border-2 transition-all duration-200 transform hover:scale-[1.02] cursor-pointer shadow-sm hover:shadow-md";
const ACTIVE_SELECTOR_STYLE: &str = "bg-pillbug-teal border-pillbug-teal text-white hover:bg-pillbug-teal/90";
const INACTIVE_SELECTOR_STYLE: &str = "bg-gray-50 border-gray-200 text-gray-700 hover:bg-gray-100 hover:border-gray-300 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:border-gray-500";


#[component]
pub fn PeriodSelector(
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
pub fn GamesTypesSelector(
    included_game_types: ReadSignal<String>,
    set_included_game_types: WriteSignal<String>,
) -> impl IntoView {
    let game_types = vec!["Full", "Base", "Full & Base"];

    view! {
        <div class="flex gap-2 mb-4 flex-wrap">
            {game_types
                .into_iter()
                .map(|gt| {
                    let gt = gt.to_string();
                    view! {
                        <button
                            type="button"
                            class={
                                let gt2 = gt.clone();
                                move || {
                                    if included_game_types.get() == gt2.clone() {
                                        format!("{} {}", BASE_SELECTOR_STYLE, ACTIVE_SELECTOR_STYLE)
                                    } else {
                                        format!(
                                            "{} {}",
                                            BASE_SELECTOR_STYLE,
                                            INACTIVE_SELECTOR_STYLE,
                                        )
                                    }
                                }
                            }
                            on:click=move |e| {
                                e.prevent_default();
                                set_included_game_types.set(gt.clone())
                            }
                        >

                            {gt.clone()}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}


#[component]
pub fn SpeedSelector(
    speeds: Vec<String>,
    selected_speed: ReadSignal<String>,
    set_selected_speed: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="flex gap-2 mb-4 flex-wrap justify-center">
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
                                    format!("{} {}", BASE_SELECTOR_STYLE, INACTIVE_SELECTOR_STYLE)
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
    }
}


#[component]
pub fn IncludeBotGamesSelector(
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