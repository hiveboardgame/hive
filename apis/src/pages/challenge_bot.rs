use crate::providers::ApiRequestsProvider;
use crate::{
    common::ChallengeAction,
    components::atoms::{
        create_challenge_button::CreateChallengeButton, simple_switch::SimpleSwitch,
    },
};
use hive_lib::{ColorChoice, GameType};
use leptos::prelude::*;
use shared_types::{ChallengeDetails, ChallengeVisibility, TimeMode};

#[component]
pub fn ChallengeBot() -> impl IntoView {
    let expansions = RwSignal::new(true);
    let api = expect_context::<ApiRequestsProvider>().0;
    let create_challenge = Callback::new(move |color_choice| {
        let api = api.get();
        let details = ChallengeDetails {
            rated: false,
            game_type: if expansions.get_untracked() {
                GameType::MLP
            } else {
                GameType::Base
            },
            visibility: ChallengeVisibility::Direct,
            // Do we hardcode bot account name and use a direct challenge
            opponent: Some(String::from("NokaBot")),
            color_choice,
            time_mode: TimeMode::Untimed,
            time_base: None,
            time_increment: None,
            band_upper: None,
            band_lower: None,
        };
        // Call bot challenge. Do we want to reuse challenge details or dose it get its own thing
        let challenge_action = ChallengeAction::Create(details);
        api.challenge(challenge_action);
    });

    view! {
        <div class="flex flex-col items-center w-72 xs:m-2 xs:w-80 sm:w-96">
            <div class="flex gap-1 p-1">Play an unrated game vs our bot</div>
            <div class="flex gap-1 p-1">
                Base <SimpleSwitch checked=expansions />MLP
            </div>

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
        </div>
    }
}
