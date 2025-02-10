use leptos::ev::Event;
use leptos::prelude::*;
use std::str::FromStr;

pub fn update_from_input(signal_to_update: RwSignal<String>) -> impl Fn(web_sys::Event) + Clone {
    move |evt: web_sys::Event| signal_to_update.update(|v| *v = event_target_value(&evt))
}

pub fn update_from_input_parsed<T>(signal_to_update: RwSignal<T>) -> impl Fn(Event) + Clone
where
    T: FromStr + Send + Sync + 'static,
{
    move |evt: Event| {
        if let Ok(value) = event_target_value(&evt).parse::<T>() {
            signal_to_update.update(|v| *v = value);
        }
    }
}
