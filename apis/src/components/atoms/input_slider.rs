use crate::components::update_from_event::update_from_input_parsed;
use leptos::leptos_dom::helpers::debounce;
use leptos::*;
use std::time::Duration;

#[component]
pub fn InputSlider(
    signal_to_update: RwSignal<i32>,
    name: &'static str,
    #[prop(into)] min: MaybeSignal<i32>,
    #[prop(into)] max: MaybeSignal<i32>,
    step: i32,
) -> impl IntoView {
    view! {
        <input
            on:input=debounce(Duration::from_millis(50), update_from_input_parsed(signal_to_update))

            type="range"
            name=name
            min=min
            max=max
            prop:value=signal_to_update
            step=step
            class="p-1 h-4 rounded-full appearance-none accent-gray-500 dark:accent-gray-400 bg-odd-light dark:bg-gray-700"
        />
    }
}
