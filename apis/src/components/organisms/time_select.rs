use crate::{
    common::TimeSignals,
    components::{
        atoms::{input_slider::InputSlider, rating::icon_for_speed, select_options::SelectOption},
        update_from_event::update_from_input_parsed,
    },
};
use leptos::ev::Event;
use leptos::*;
use leptos_icons::*;
use shared_types::{GameSpeed, TimeMode};

#[component]
pub fn TimeSelect(
    title: &'static str,
    children: ChildrenFn,
    time_signals: TimeSignals,
    on_change: Callback<Event>,
) -> impl IntoView {
    let gamespeed_icon = move || {
        let speed = match time_signals.time_control.get() {
            TimeMode::Untimed => GameSpeed::Untimed,
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => GameSpeed::from_base_increment(
                Some(time_signals.total_seconds.get()),
                Some(time_signals.sec_per_move.get()),
            ),
        };
        view! { <Icon width="50" height="50" class="p-2" icon=icon_for_speed(&speed)/> }
    };

    view! {
        <div class="flex">
            <label class="mr-1">
                <div class="flex items-center">
                    {gamespeed_icon} <p class="text-3xl font-extrabold">{title}</p>
                </div>
                Time Control:
                <select
                    class="bg-odd-light dark:bg-gray-700"
                    name="Time Control"
                    on:change=on_change
                >
                    {children}

                </select>
            </label>
        </div>
        <Show when=move || time_signals.time_control.get() != TimeMode::Untimed>
            <Show
                when=move || time_signals.time_control.get() == TimeMode::RealTime
                fallback=move || {
                    view! {
                        <div class="flex flex-col justify-center">

                            <label class="flex-col items-center">
                                <div class="flex gap-1 p-1">
                                    <select
                                        class="mr-1 bg-odd-light dark:bg-gray-700"
                                        name="Correspondence Mode"
                                        on:change=update_from_input_parsed(time_signals.corr_mode)
                                    >

                                        <SelectOption
                                            value=time_signals.corr_mode
                                            is="Days per move"
                                        />
                                        <SelectOption
                                            value=time_signals.corr_mode
                                            is="Total time each"
                                        />

                                    </select>
                                    <div class="w-4">{time_signals.corr_days}</div>
                                </div>
                                <InputSlider
                                    signal_to_update=time_signals.corr_days
                                    name="Correspondence"
                                    min=1
                                    max=14
                                    step=1
                                />
                            </label>
                        </div>
                    }
                }
            >

                <div class="flex flex-col justify-center">
                    <label class="flex-col items-center">
                        <div>
                            {move || {
                                format!(
                                    "Minutes per side: {}",
                                    time_signals.total_seconds.get() / 60,
                                )
                            }}

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
                            {move || {
                                format!("Increment in sec: {}", time_signals.sec_per_move.get())
                            }}

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
        </Show>
    }
}
