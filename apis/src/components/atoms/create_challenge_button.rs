use crate::providers::Config;
use hive_lib::ColorChoice;
use leptos::{prelude::*, text_prop::TextProp};
use leptos_icons::*;

#[component]
pub fn CreateChallengeButton(
    color_choice: StoredValue<ColorChoice>,
    create_challenge: Callback<ColorChoice>,
) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let icon_data = |color_choice: ColorChoice| {
        match color_choice {
            ColorChoice::Random => {
                (icondata::BsHexagonHalf,"w-full h-full stroke-1 stroke-black")
            }
            ColorChoice::White => {
                if config().prefers_dark {
                    (icondata::BsHexagonFill,"w-full h-full fill-white")
                } else {
                    (icondata::BsHexagon,"w-full h-full stroke-1 stroke-black")
                }
            }
            ColorChoice::Black => {
                if config().prefers_dark {
                    (icondata::BsHexagon, "w-full h-full stroke-1 stroke-white")
                } else {
                    (icondata::BsHexagonFill,"w-full h-full fill-black")
                }
            }
        }
    };
    view! {
        <button
            title=color_choice.get_value().to_string()
            formmethod="dialog"
            class=format!(
                "m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 my-1 p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded",
            )

            on:click=move |_| { create_challenge.run(color_choice.get_value()) }
        >
        {
            let (icon, class) = icon_data(color_choice.get_value());
            view! {
                <Icon
                    icon
                    attr:class=class
                />
            }
        }
        </button>
    }
}
