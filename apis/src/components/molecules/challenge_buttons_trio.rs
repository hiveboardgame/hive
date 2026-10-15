use crate::{components::atoms::create_challenge_button::CreateChallengeButton, i18n::*};
use hive_lib::ColorChoice;
use leptos::prelude::*;

#[component]
pub fn ChallengeButtonsTrio(create_challenge: Callback<ColorChoice>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex flex-col gap-2 justify-center items-center">

            <p class="text-sm text-center text-gray-700 dark:text-gray-300">
                {move || t_string!(i18n, home.pick_your_color_and_create_a_challenge)}
            </p>
            <form method="dialog" class="flex justify-center items-baseline">
                <CreateChallengeButton
                    color_choice=StoredValue::new(ColorChoice::White)
                    create_challenge
                />
                <CreateChallengeButton
                    color_choice=StoredValue::new(ColorChoice::Random)
                    create_challenge
                />
                <CreateChallengeButton
                    color_choice=StoredValue::new(ColorChoice::Black)
                    create_challenge
                />
            </form>
        </div>
    }
}
