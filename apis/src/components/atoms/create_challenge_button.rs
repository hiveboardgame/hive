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
            class="m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 my-1 p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded"

            on:click=move |_| { create_challenge.run(color_choice.get_value()) }
        >
            <Icon icon style="height:100%; width:100%" />
        </button>
    }
}
