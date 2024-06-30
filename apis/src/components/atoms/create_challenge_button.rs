use crate::providers::ColorScheme;
use hive_lib::ColorChoice;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn CreateChallengeButton(
    color_choice: StoredValue<ColorChoice>,
    create_challenge: Callback<ColorChoice>,
) -> impl IntoView {
    let color_context = expect_context::<ColorScheme>;
    let icon = move |color_choice: ColorChoice| {
        move || match color_choice {
            ColorChoice::Random => {
                view! { <Icon icon=icondata::BsHexagonHalf class="w-full h-full"/> }
            }
            ColorChoice::White => {
                if (color_context().prefers_dark)() {
                    view! { <Icon icon=icondata::BsHexagonFill class="w-full h-full fill-white"/> }
                } else {
                    view! { <Icon icon=icondata::BsHexagon class="w-full h-full stroke-1 stroke-black"/> }
                }
            }
            ColorChoice::Black => {
                if (color_context().prefers_dark)() {
                    view! { <Icon icon=icondata::BsHexagon class="w-full h-full stroke-1 stroke-white"/> }
                } else {
                    view! { <Icon icon=icondata::BsHexagonFill class="w-full h-full fill-black"/> }
                }
            }
        }
    };
    view! {
        <button
            title=color_choice().to_string()
            class=format!(
                "m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 my-1 p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded",
            )

            on:click=move |_| { create_challenge(color_choice()) }
        >
            {icon(color_choice())}
        </button>
    }
}
