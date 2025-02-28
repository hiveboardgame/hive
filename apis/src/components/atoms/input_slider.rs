use leptos::prelude::*;

const STYLE: &str = "block mt-2 w-full rounded-lg hover:pillbug-teal accent-orange-twilight active:accent-pillbug-teal";

#[component]
pub fn InputSlider(
    signal_to_update: RwSignal<i32>,
    name: &'static str,
    #[prop(into)] min: Signal<i32>,
    #[prop(into)] max: Signal<i32>,
    step: i32,
) -> impl IntoView {
    view! {
        <input  
            type="range" 
            class=STYLE
            name=name
            min=min
            max=max
            step=step
            value=signal_to_update
            on:input:target=move |ev| {
                let val = ev.target().value().parse::<i32>().unwrap();
                signal_to_update.set(val);
            }
            />
    }
}
