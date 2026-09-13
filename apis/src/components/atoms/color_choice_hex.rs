use hive_lib::ColorChoice;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ColorChoiceHex(
    color_choice: ColorChoice,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    // leptos_icons can wrap the path in a `<g fill="currentColor">`, which blocks any
    // `fill-*` set on the `<svg>`, so the tile colour has to come from `color`.
    let (icon, color) = match color_choice {
        ColorChoice::White => (icondata_bs::BsHexagonFill, "text-white"),
        ColorChoice::Black => (icondata_bs::BsHexagonFill, "text-black"),
        ColorChoice::Random => (icondata_bs::BsHexagonHalf, ""),
    };
    view! {
        <Icon
            icon
            attr:class=format!(
                "shrink-0 stroke-1 stroke-black dark:stroke-white {color} {extend_tw_classes}",
            )
        />
    }
}
