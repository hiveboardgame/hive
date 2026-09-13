use crate::components::atoms::color_choice_hex::ColorChoiceHex;
use hive_lib::ColorChoice;
use leptos::prelude::*;

#[component]
pub fn CreateChallengeButton(
    color_choice: StoredValue<ColorChoice>,
    create_challenge: Callback<ColorChoice>,
) -> impl IntoView {
    view! {
        <button
            title=color_choice.get_value().to_string()
            formmethod="dialog"
            type="submit"
            class="m-0.5 w-14 h-16 shrink-0 xs:m-1 xs:w-16 xs:h-[4.5rem] ui-button ui-button-secondary ui-button-tiny"

            on:click=move |_| { create_challenge.run(color_choice.get_value()) }
        >
            <ColorChoiceHex color_choice=color_choice.get_value() extend_tw_classes="size-full" />
        </button>
    }
}
