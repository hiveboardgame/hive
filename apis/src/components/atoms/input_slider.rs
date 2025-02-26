use leptos::prelude::*;
use leptos::callback::Callback;
#[component]
pub fn InputSlider(
    signal_to_update: RwSignal<i32>,
    name: &'static str,
    #[prop(into)] min: Signal<i32>,
    #[prop(into)] max: Signal<i32>,
    step: i32,
) -> impl IntoView {
    let min = Signal::derive(move || min() as f64);
    let max = Signal::derive(move || max() as f64);
    let step: f64 = step as f64;
    let default_value = MaybeProp::derive(move || vec![signal_to_update() as f64].into());
    let on_value_change= Callback::<Vec<f64>, _>::new(move |val| {
        signal_to_update.update(|v| *v = val[0] as i32);
    });
    view! {
        /*<SliderRoot
            min
            max
            step
            on_value_change
            default_value
            attr:class="flex relative items-center w-fit min-w-[150px] h-5 select-none touch-none"
            attr:name=name
        >
        <SliderTrack attr:class="bg-white relative grow rounded-full h-[3px]">
        <SliderRange attr:class="absolute h-full rounded-full bg-orange-twilight">
            {().into_view()}
        </SliderRange>
    </SliderTrack>
    <SliderThumb attr:class="bg-button-dawn dark:bg-button-twilight block w-5 h-5 shadow-lg rounded-[10px] hover:bg-pillbug-teal">
        {().into_view()}
    </SliderThumb>
        </SliderRoot>
        */
        "FIX SLIDERS"
    }
}
