use crate::i18n::*;
use crate::{
    common::TimeSignals,
    components::atoms::{input_slider::InputSlider, rating::icon_for_speed},
};
use leptix_primitives::components::radio_group::{RadioGroupItem, RadioGroupRoot};
use leptos::*;
use leptos_icons::*;
use shared_types::{CorrespondenceMode, GameSpeed, TimeMode};
use std::str::FromStr;

#[component]
pub fn TimeSelect(
    is_tournament: bool,
    time_signals: TimeSignals,
    on_value_change: Callback<String>,
    allowed_values: Vec<TimeMode>,
) -> impl IntoView {
    let i18n = use_i18n();
    let title = if is_tournament {
        "Match settings:".into_view()
    } else {
        t!(i18n, home.custom_game.title).into_view()
    };
    let time_mode = move || time_signals.time_mode.get();
    let gamespeed_icon = move || {
        let speed = match time_mode() {
            TimeMode::Untimed => GameSpeed::Untimed,
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => GameSpeed::from_base_increment(
                Some(time_signals.total_seconds.get()),
                Some(time_signals.sec_per_move.get()),
            ),
        };
        view! { <Icon width="50" height="50" class="p-2" icon=icon_for_speed(&speed)/> }
    };
    let radio_style = "flex items-center p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded data-[state=checked]:bg-button-dawn dark:data-[state=checked]:bg-button-twilight data-[state=unchecked]:bg-odd-light dark:data-[state=unchecked]:bg-gray-700 data-[state=unchecked]:bg-odd-light dark:data-[state=unchecked]:bg-gray-700";
    let allow_realtime = allowed_values.contains(&TimeMode::RealTime);
    let allow_correspondence = allowed_values.contains(&TimeMode::Correspondence);
    let allow_untimed = allowed_values.contains(&TimeMode::Untimed);
    view! {
        <div class="flex flex-col p-2">
            <div class="flex items-center">
                {gamespeed_icon} <p class="text-3xl font-extrabold">{title}</p>
            </div>

            <RadioGroupRoot
                required=true
                attr:class="flex flex-row gap-2 justify-center"
                default_value=MaybeProp::derive(move || Some(
                    time_signals.time_mode.get().to_string(),
                ))

                on_value_change
            >
                <Show when=move || allow_realtime>
                    <RadioGroupItem value="Real Time" attr:class=radio_style>
                        {t!(i18n, home.custom_game.mode.real_time.title)}
                    </RadioGroupItem>
                </Show>
                <Show when=move || allow_correspondence>
                    <RadioGroupItem value="Correspondence" attr:class=radio_style>
                        {t!(i18n, home.custom_game.mode.correspondence.title)}
                    </RadioGroupItem>
                </Show>
                <Show when=move || allow_untimed>
                    <RadioGroupItem value="Untimed" attr:class=radio_style>
                        {t!(i18n, home.custom_game.mode.untimed)}
                    </RadioGroupItem>
                </Show>
            </RadioGroupRoot>

        </div>
        <Show when=move || time_mode() == TimeMode::RealTime>
            <div class="flex flex-col justify-center">
                <label class="flex-col items-center">
                    <div>
                        {t!(
                            i18n, home.custom_game.mode.real_time.minutes_per_side, count = move ||
                            time_signals.total_seconds.get() / 60
                        )}

                    </div>
                    <InputSlider
                        signal_to_update=time_signals.step_min
                        name="minutes"
                        min=1
                        max=32
                        step=1
                    />
                </label>
                <label class="flex-col items-center">
                    <div>
                        {t!(
                            i18n, home.custom_game.mode.real_time.increment_in_seconds, count = move
                            || time_signals.sec_per_move.get()
                        )}

                    </div>
                    <InputSlider
                        signal_to_update=time_signals.step_sec
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
                <RadioGroupRoot
                    required=true
                    attr:class="flex flex-row gap-2 p-2"
                    default_value=MaybeProp::derive(move || Some(
                        time_signals.corr_mode.get().to_string(),
                    ))

                    on_value_change=move |v: String| {
                        if let Ok(new_value) = CorrespondenceMode::from_str(&v) {
                            time_signals.corr_mode.update(|v| *v = new_value)
                        }
                    }
                >

                    <RadioGroupItem value="Days per move" attr:class=radio_style>
                        {t!(i18n, home.custom_game.mode.correspondence.days_per_move)}
                    </RadioGroupItem>
                    <RadioGroupItem value="Total time each" attr:class=radio_style>
                        {t!(i18n, home.custom_game.mode.correspondence.total_time_each)}
                    </RadioGroupItem>
                </RadioGroupRoot>
                <div class="flex flex-row gap-4 p-3">
                    <InputSlider
                        signal_to_update=time_signals.corr_days
                        name="CorrespondenceSlider"
                        min=1
                        max=14
                        step=1
                    />
                    <div class="flex">
                        {t!(
                            i18n, home.custom_game.mode.correspondence.time, count = move ||
                            time_signals.corr_days.get()
                        )}

                    </div>
                </div>
            </div>
        </Show>
    }
}
