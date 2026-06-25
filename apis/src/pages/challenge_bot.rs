use crate::{
    common::ChallengeAction,
    components::{
        atoms::simple_switch::SimpleSwitch,
        molecules::challenge_buttons_trio::ChallengeButtonsTrio,
    },
    providers::ApiRequestsProvider,
};
use hive_lib::GameType;
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
    let api = expect_context::<ApiRequestsProvider>().0;

    let radio_style = move |active: bool| {
        if active {
            "ui-choice ui-choice-compact ui-choice-active cursor-pointer"
        } else {
            "ui-choice ui-choice-compact ui-choice-inactive cursor-pointer"
        }
    };

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
            opponent: Some(difficulty.get_untracked().to_bot_name()),
            color_choice,
            time_mode: TimeMode::Untimed,
            time_base: None,
            time_increment: None,
            band_upper: None,
            band_lower: None,
        };
        let challenge_action = ChallengeAction::Create(details);
        api.challenge(challenge_action);
    });

    view! {
        <div class="flex flex-col gap-4 items-center max-w-sm box-border w-[calc(100vw_-_3rem)] xs:m-2">
            <div class="text-sm font-medium text-gray-700 dark:text-gray-300">
                Play an unrated game vs our bot
            </div>
            <div class="ui-setting-group">
                <div class="flex gap-2 items-center text-sm font-medium text-gray-700 dark:text-gray-300">
                    <span>Base</span>
                    <SimpleSwitch checked=expansions />
                    <span>MLP</span>
                </div>
            </div>

            <div class="flex flex-col items-center w-full">
                <div class="flex flex-wrap gap-2 justify-center p-2 w-full">
                    <button
                        type="button"
                        class=move || radio_style(matches!(difficulty.get(), BotDifficulty::Easy))
                        on:click=move |_| difficulty.set(BotDifficulty::Easy)
                    >
                        Easy
                    </button>
                    <button
                        type="button"
                        class=move || radio_style(matches!(difficulty.get(), BotDifficulty::Medium))
                        on:click=move |_| difficulty.set(BotDifficulty::Medium)
                    >
                        Medium
                    </button>
                    <button
                        type="button"
                        class=move || radio_style(matches!(difficulty.get(), BotDifficulty::Hard))
                        on:click=move |_| difficulty.set(BotDifficulty::Hard)
                    >
                        Hard
                    </button>
                </div>
            </div>
            <ChallengeButtonsTrio create_challenge />
        </div>
    }
}
