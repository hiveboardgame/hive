use leptos::prelude::*;
use hive_lib::ColorChoice;
use crate::{
    components::atoms::{
        create_challenge_button::CreateChallengeButton
    },
};

#[component]
pub fn ChallengeButtonsTrio(
    create_challenge: Callback<ColorChoice>
) -> impl IntoView {
    view! {
        
        <body class="flex flex-col gap-2 justify-center items-center">

                <p class="text-sm text-center text-gray-700 dark:text-gray-300">"Pick your color and create a challenge:"</p>

                <div class="flex justify-center items-baseline">
                        <form method="dialog">
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
        </body>
    }
}
