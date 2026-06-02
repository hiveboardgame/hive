use crate::{
    components::molecules::hiveground_stack::HivegroundStack,
    hiveground::{
        layers_by_position,
        layers_for_position,
        stack_positions,
        HivegroundInteraction,
        HivegroundPaint,
        HivegroundRenderModel,
    },
};
use leptos::prelude::*;

#[component]
pub fn HivegroundStacks(
    model: Memo<HivegroundRenderModel>,
    paint: Memo<HivegroundPaint>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let positions = stack_positions(model);
    let layers_by_position = layers_by_position(model);

    view! {
        <For each=move || positions() key=|position| *position let(position)>
            {
                let layers = layers_for_position(layers_by_position, position);
                view! { <HivegroundStack position layers paint interaction /> }
            }
        </For>
    }
}
