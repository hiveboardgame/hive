use crate::websocket::new_style::client::ClientApi;
use crate::{
    common::ChallengeAction,
    components::atoms::{
        create_challenge_button::CreateChallengeButton, simple_switch::SimpleSwitch,
    },
};
use hive_lib::{ColorChoice, GameType};
use leptos::prelude::*;
use shared_types::{ChallengeDetails, ChallengeVisibility, TimeMode};

#[derive(Clone, Copy)]
enum BotDifficulty {
    Easy,
    Medium,
    Hard,
}

impl BotDifficulty {
    fn to_bot_name(self) -> String {
        match self {
            BotDifficulty::Easy => "nokamute-easy",
            BotDifficulty::Medium => "nokamute-medium",
            BotDifficulty::Hard => "nokamute-hard",
        }
        .to_string()
    }
}

#[component]
pub fn ChallengeBot() -> impl IntoView {
    let expansions = RwSignal::new(true);
    let difficulty = RwSignal::new(BotDifficulty::Medium);
    let client_api = expect_context::<ClientApi>();

    let radio_style = move |active: bool| {
        format!("flex items-center p-2 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded cursor-pointer {}", 
            if active {
                "bg-button-dawn dark:bg-button-twilight"
            } else {
                "dark:bg-gray-700 bg-odd-light"
            }
        )
    };

    let create_challenge = Callback::new(move |color_choice| {
        let details = ChallengeDetails {
            rated: false,
            game_type: if expansions.get_untracked() {
                GameType::MLP
            } else {
                GameType::Base
            },
            visibility: ChallengeVisibility::Direct,
            opponent: Some(difficulty.get_untracked().to_bot_name()),
            color_choice,
            time_mode: TimeMode::Untimed,
            time_base: None,
            time_increment: None,
            band_upper: None,
            band_lower: None,
        };
        let challenge_action = ChallengeAction::Create(details);
        let api = client_api;
        api.challenge(challenge_action);
    });

    view! {
        <div class="flex flex-col items-center w-72 xs:m-2 xs:w-80 sm:w-96">
            <div class="flex gap-1 p-1">Play an unrated game vs our bot</div>
            <div class="flex gap-1 p-1">Base <SimpleSwitch checked=expansions />MLP</div>

            <div class="flex flex-col items-center">
                <div class="flex gap-2 justify-center p-2">
                    <div
                        class=move || radio_style(matches!(difficulty.get(), BotDifficulty::Easy))
                        on:click=move |_| difficulty.set(BotDifficulty::Easy)
                    >
                        Easy
                    </div>
                    <div
                        class=move || radio_style(matches!(difficulty.get(), BotDifficulty::Medium))
                        on:click=move |_| difficulty.set(BotDifficulty::Medium)
                    >
                        Medium
                    </div>
                    <div
                        class=move || radio_style(matches!(difficulty.get(), BotDifficulty::Hard))
                        on:click=move |_| difficulty.set(BotDifficulty::Hard)
                    >
                        Hard
                    </div>
                </div>
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
