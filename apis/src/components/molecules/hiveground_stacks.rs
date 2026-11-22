use crate::{
    components::molecules::hiveground_stack::{HivegroundStack, StackCountBadge},
    hiveground::{
        layers_by_position,
        layers_for_position,
        stack_positions,
        HivegroundInteraction,
        HivegroundPaint,
        HivegroundRenderModel,
        LayersByPosition,
    },
};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn HivegroundStacks(
    model: Memo<HivegroundRenderModel>,
    paint: Memo<HivegroundPaint>,
    interaction: HivegroundInteraction,
    #[prop(optional)] show_count: Signal<bool>,
    #[prop(default = 1.0)] badge_scale: f32,
) -> impl IntoView {
    let positions = stack_positions(model);
    let layers_by_position = layers_by_position(model);

    view! {
        <For each=positions key=|position| *position let(position)>
            {
                let layers = layers_for_position(layers_by_position, position);
                view! { <HivegroundStack position layers paint interaction /> }
            }
        </For>
        <Show when=move || show_count.get()>
            <StackCountBadges positions layers_by_position paint interaction badge_scale />
        </Show>
    }
}

#[component]
fn StackCountBadges(
    positions: Signal<Vec<Position>>,
    layers_by_position: Memo<LayersByPosition>,
    paint: Memo<HivegroundPaint>,
    interaction: HivegroundInteraction,
    badge_scale: f32,
) -> impl IntoView {
    view! {
        <For each=positions key=|position| *position let(position)>
            {
                let layers = layers_for_position(layers_by_position, position);
                view! { <StackCountBadge position layers paint interaction badge_scale /> }
            }
        </For>
    }
}
