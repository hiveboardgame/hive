use crate::providers::Config;
use hive_lib::ColorChoice;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn CreateChallengeButton(
    color_choice: StoredValue<ColorChoice>,
    create_challenge: Callback<ColorChoice>,
) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let icon = Signal::derive(move || {
        config.with(|cfg| match (color_choice.get_value(), cfg.prefers_dark) {
            (ColorChoice::Random, _) => icondata_bs::BsHexagonHalf,
            (ColorChoice::White, false) | (ColorChoice::Black, true) => icondata_bs::BsHexagon,
            _ => icondata_bs::BsHexagonFill,
        })
    });
    view! {
        <button
            title=color_choice.get_value().to_string()
            formmethod="dialog"
            type="submit"
            class="m-0.5 w-14 h-16 shrink-0 xs:m-1 xs:w-16 xs:h-[4.5rem] ui-button ui-button-secondary ui-button-tiny"

            on:click=move |_| { create_challenge.run(color_choice.get_value()) }
        >
            <Icon icon style="height:100%; width:100%" />
        </button>
    }
}
