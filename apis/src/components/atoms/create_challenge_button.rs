use crate::providers::Config;
use hive_lib::ColorChoice;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn CreateChallengeButton(
    color_choice: StoredValue<ColorChoice>,
    create_challenge: Callback<ColorChoice>,
    #[prop(optional)] disabled: Signal<bool>,
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
            prop:disabled=disabled
            class=move || {
                if disabled() {
                    "p-1 m-1 my-1 w-16 rounded h-[4.5rem] dark:bg-gray-700 bg-odd-light drop-shadow-lg dark:shadow-gray-600 opacity-40 cursor-not-allowed"
                } else {
                    "p-1 m-1 my-1 w-16 rounded transition-transform duration-300 transform dark:bg-gray-700 hover:shadow-xl active:scale-95 h-[4.5rem] bg-odd-light drop-shadow-lg dark:hover:shadow dark:hover:shadow-gray-500 dark:shadow-gray-600"
                }
            }
            on:click=move |_| {
                if !disabled() {
                    create_challenge.run(color_choice.get_value())
                }
            }
        >
            <Icon icon style="height:100%; width:100%" />
        </button>
    }
}
