use crate::i18n::*;
use crate::{
    common::ChallengeAction,
    components::{atoms::rating::icon_for_speed, molecules::modal::Modal},
    pages::challenge_create::ChallengeCreate,
    providers::{ApiRequests, AuthContext},
};
use core::panic;
use hive_lib::{ColorChoice, GameType};
use leptos::{html::Dialog, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use shared_types::ChallengeVisibility;
use shared_types::{ChallengeDetails, GameSpeed::*, TimeMode};

pub enum QuickPlayTimeControl {
    Bullet1p2,
    Blitz3p3,
    Blitz5p4,
    Rapid10p10,
    Rapid15p10,
    Classic20p20,
    Classic30p30,
}
use QuickPlayTimeControl::*;
const BUTTON_STYLE: &str = "flex w-full gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[component]
pub fn GridButton(time_control: QuickPlayTimeControl) -> impl IntoView {
    let (display_text, icon_data, base, increment) = match time_control {
        Bullet1p2 => ("1+2".to_owned(), icon_for_speed(&Bullet), 1, 2),
        Blitz3p3 => ("3+3".to_owned(), icon_for_speed(&Blitz), 3, 3),
        Blitz5p4 => ("5+4".to_owned(), icon_for_speed(&Blitz), 5, 4),
        Rapid10p10 => ("10+10".to_owned(), icon_for_speed(&Rapid), 10, 10),
        Rapid15p10 => ("15+10".to_owned(), icon_for_speed(&Rapid), 15, 10),
        Classic20p20 => ("20+20".to_owned(), icon_for_speed(&Classic), 20, 20),
        Classic30p30 => ("30+30".to_owned(), icon_for_speed(&Classic), 30, 30),
    };
    view! {
        <button
            class=BUTTON_STYLE
            on:click=move |_| {
                let auth_context = expect_context::<AuthContext>();
                let account = match auth_context.user.get() {
                    Some(Ok(account)) => Some(account),
                    _ => None,
                };
                if account.is_some() {
                    let api = ApiRequests::new();
                    let details = ChallengeDetails {
                        rated: true,
                        game_type: GameType::MLP,
                        visibility: ChallengeVisibility::Public,
                        opponent: None,
                        color_choice: ColorChoice::Random,
                        time_mode: TimeMode::RealTime,
                        time_base: Some(base * 60),
                        time_increment: Some(increment),
                        band_upper: None,
                        band_lower: None,
                    };
                    let challenge_action = ChallengeAction::Create(details);
                    api.challenge(challenge_action);
                } else {
                    let navigate = use_navigate();
                    navigate("/login", Default::default());
                }
            }
        >

            <Icon icon=icon_data />
            {display_text}
        </button>
    }
}
#[component]
pub fn QuickPlay() -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(false);
    let dialog_el = NodeRef::<Dialog>::new();
    view! {
        <div class="flex flex-col items-center m-2 grow">
            <Modal open dialog_el=dialog_el>
                <ChallengeCreate open />
            </Modal>
            <span class="flex justify-center mb-4 text-xl font-bold">
                {t!(i18n, home.create_game)}
            </span>
            <div class="grid grid-cols-2 gap-2 place-items-center w-full sm:gap-4 sm:grid-cols-3">
                <GridButton time_control=Bullet1p2 />
                <GridButton time_control=Blitz3p3 />
                <GridButton time_control=Blitz5p4 />
                <GridButton time_control=Rapid10p10 />
                <GridButton time_control=Classic20p20 />
                <button
                    class=BUTTON_STYLE
                    on:click=move |_| {
                        let auth_context = expect_context::<AuthContext>();
                        let account = match auth_context.user.get() {
                            Some(Ok(account)) => Some(account),
                            _ => None,
                        };
                        if account.is_some() {
                            open.update(move |b| *b = true)
                        } else {
                            let navigate = use_navigate();
                            navigate("/login", Default::default());
                        }
                    }
                >

                    {t!(i18n, home.custom_game.button)}
                </button>
            </div>
        </div>
    }
}
