use crate::components::atoms::input_slider::InputSliderWithCallback;
use crate::i18n::*;
use crate::providers::{ChallengeParams, ChallengeParamsStoreFields};
use crate::{common::TimeParamsStoreFields, components::atoms::rating::icon_for_speed};
use leptos::prelude::*;
use leptos_icons::*;
use reactive_stores::Store;
use shared_types::{CorrespondenceMode, GameSpeed, TimeMode};
#[component]
pub fn TimeSelect(
    is_tournament: bool,
    params: Store<ChallengeParams>,
    on_value_change: Callback<TimeMode>,
    allowed_values: Vec<TimeMode>,
) -> impl IntoView {
    let i18n = use_i18n();
    let title = move || {
        if is_tournament {
            "Match settings:"
        } else {
            t_string!(i18n, home.custom_game.title)
        }
    };
    let time_mode = move || params.time_signals().time_mode().get();
    let corr_mode = move || params.time_signals().corr_mode().get();
    let gamespeed_icon = move || {
        let speed = match time_mode() {
            TimeMode::Untimed => GameSpeed::Untimed,
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => {
                let time_data = params.time_signals().get();
                GameSpeed::from_base_increment(
                    Some(time_data.total_seconds()),
                    Some(time_data.sec_per_move()),
                )
            }
        };
        view! { <Icon width="50" height="50" attr:class="p-2" icon=icon_for_speed(speed) /> }
    };
    let radio_style = |active| {
        format!("flex items-center p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded {}", 
        if active {
            "bg-button-dawn dark:bg-button-twilight"
        } else {
            "dark:bg-gray-700 bg-odd-light "
        }
    )
    };
    let allow_realtime = allowed_values.contains(&TimeMode::RealTime);
    let allow_correspondence = allowed_values.contains(&TimeMode::Correspondence);
    let allow_untimed = allowed_values.contains(&TimeMode::Untimed);
    let toggle_time_mode = move |t: TimeMode| {
        params.time_signals().time_mode().update(|v| *v = t);
        on_value_change.run(t);
    };
    let toggle_corr_mode = move |t: CorrespondenceMode| {
        params.time_signals().corr_mode().update(|v| *v = t);
    };
    let minutes_calllback = Callback::new(move |new: i32| {
        params.time_signals().step_min().update(|v| *v = new);
    });
    let seconds_calllback = Callback::new(move |new: i32| {
        params.time_signals().step_sec().update(|v| *v = new);
    });
    let days_calllback = Callback::new(move |new: i32| {
        params.time_signals().corr_days().update(|v| *v = new);
    });
    view! {
        <div class="flex flex-col p-2">
            <div class="flex items-center">
                {gamespeed_icon} <p class="text-3xl font-extrabold">{title}</p>
            </div>

            <div class="flex flex-row gap-2 justify-center">
                <Show when=move || allow_realtime>
                    <div
                        on:click=move |_| toggle_time_mode(TimeMode::RealTime)
                        class=move || radio_style(time_mode() == TimeMode::RealTime)
                    >
                        {t!(i18n, home.custom_game.mode.real_time.title)}
                    </div>
                </Show>
                <Show when=move || allow_correspondence>
                    <div
                        on:click=move |_| toggle_time_mode(TimeMode::Correspondence)
                        class=move || radio_style(time_mode() == TimeMode::Correspondence)
                    >
                        {t!(i18n, home.custom_game.mode.correspondence.title)}
                    </div>
                </Show>
                <Show when=move || allow_untimed>
                    <div
                        on:click=move |_| toggle_time_mode(TimeMode::Untimed)
                        class=move || radio_style(time_mode() == TimeMode::Untimed)
                    >
                        {t!(i18n, home.custom_game.mode.untimed)}
                    </div>
                </Show>
            </div>
        </div>
        <Show when=move || time_mode() == TimeMode::RealTime>
            <div class="flex flex-col justify-center">
                <label class="flex-col items-center">
                    <div>
                        {t!(
                            i18n, home.custom_game.mode.real_time.minutes_per_side, count = move ||
                                    params.time_signals().with(|ts| ts.total_seconds() / 60)
                        )}

                    </div>
                    <InputSliderWithCallback
                        signal=Signal::derive(move || params.time_signals().step_min().get())
                        name="minutes"
                        callback=minutes_calllback
                        min=1
                        max=32
                        step=1
                    />
                </label>
                <label class="flex-col items-center">
                    <div>
                        {t!(
                            i18n, home.custom_game.mode.real_time.increment_in_seconds, count = move
                                    || params.time_signals().with(|ts| ts.sec_per_move())
                        )}

                    </div>
                    <InputSliderWithCallback
                        signal=Signal::derive(move || params.time_signals().step_sec().get())
                        callback=seconds_calllback
                        name="increment"
                        min=0
                        max=32
                        step=1
                    />
                </label>
            </div>
        </Show>
        <Show when=move || time_mode() == TimeMode::Correspondence>
            <div class="flex flex-col justify-center items-center w-full">
                <div class="flex flex-row gap-2 p-2">
                    <div
                        on:click=move |_| toggle_corr_mode(CorrespondenceMode::DaysPerMove)
                        class=move || radio_style(corr_mode() == CorrespondenceMode::DaysPerMove)
                    >
                        {t!(i18n, home.custom_game.mode.correspondence.days_per_move)}
                    </div>
                    <div
                        class=move || radio_style(corr_mode() == CorrespondenceMode::TotalTimeEach)
                        on:click=move |_| toggle_corr_mode(CorrespondenceMode::TotalTimeEach)
                    >
                        {t!(i18n, home.custom_game.mode.correspondence.total_time_each)}
                    </div>
                </div>
                <div class="flex flex-row gap-4 p-3">
                    <InputSliderWithCallback
                        signal=Signal::derive(move || params.time_signals().corr_days().get())
                        callback=days_calllback
                        name="CorrespondenceSlider"
                        min=1
                        max=20
                        step=1
                    />
                    <div class="flex">
                        {t!(
                            i18n, home.custom_game.mode.correspondence.time, count = move ||
                                    params.time_signals().corr_days().get()
                        )}

                    </div>
                </div>
            </div>
        </Show>
    }
}
