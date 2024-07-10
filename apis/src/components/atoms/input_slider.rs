use leptix_primitives::components::slider::*;
use leptos::*;

#[component]
pub fn InputSlider(
    signal_to_update: RwSignal<i32>,
    name: &'static str,
    #[prop(into)] min: MaybeSignal<i32>,
    #[prop(into)] max: MaybeSignal<i32>,
    step: i32,
) -> impl IntoView {
    let min = Signal::derive(move || min() as f64);
    let max = Signal::derive(move || max() as f64);
    let step: f64 = step as f64;
    let on_value_change = Callback::from(move |val: Vec<f64>| {
        signal_to_update.set(val[0] as i32);
    });
    view! {
        <SliderRoot min max step on_value_change
            attr:class="relative flex items-center select-none touch-none w-[200px] h-5"
            attr:name=name
        >
            <SliderTrack attr:class="bg-blackA7 relative grow rounded-full h-[3px]">
                <SliderRange attr:class="absolute h-full bg-white rounded-full">
                    {().into_view()}
                </SliderRange>
            </SliderTrack>
            <SliderThumb attr:class="block w-5 h-5 bg-white shadow-[0_2px_10px] shadow-blackA4 rounded-[10px] hover:bg-violet3 focus:outline-none focus:shadow-[0_0_0_5px] focus:shadow-blackA5">
                {().into_view()}
            </SliderThumb>
        </SliderRoot>
    }
}
