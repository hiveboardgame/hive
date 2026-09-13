use crate::providers::Config;
use hive_lib::ColorChoice;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ColorChoiceHex(
    color_choice: ColorChoice,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let icon = Signal::derive(move || {
        config.with(|cfg| match (&color_choice, cfg.prefers_dark) {
            (ColorChoice::Random, _) => icondata_bs::BsHexagonHalf,
            (ColorChoice::White, false) | (ColorChoice::Black, true) => icondata_bs::BsHexagon,
            _ => icondata_bs::BsHexagonFill,
        })
    });
    view! { <Icon icon attr:class=format!("shrink-0 {extend_tw_classes}") /> }
}
