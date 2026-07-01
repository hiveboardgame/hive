use crate::{
    common::ChallengeAction,
    components::{atoms::rating::icon_for_speed, molecules::modal::Modal},
    hooks::tap_feedback::use_tap_feedback,
    i18n::*,
    pages::{challenge_bot::ChallengeBot, challenge_create::ChallengeCreate},
    providers::{challenge_params_cookie, ApiRequestsProvider, AuthContext, ChallengeParams},
};
use hudsoni::{ColorChoice, GameType};
use leptos::{ev, html::Dialog, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use leptos_use::use_event_listener;
use reactive_stores::Store;
use shared_types::{ChallengeDetails, ChallengeVisibility, GameSpeed::*, TimeMode};

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

#[component]
pub fn GridButton(time_control: QuickPlayTimeControl) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let (display_text, icon_data, base, increment, speed_name) = match time_control {
        Bullet1p2 => ("1+2".to_owned(), icon_for_speed(Bullet), 1, 2, "Bullet"),
        Blitz3p3 => ("3+3".to_owned(), icon_for_speed(Blitz), 3, 3, "Blitz"),
        Blitz5p4 => ("5+4".to_owned(), icon_for_speed(Blitz), 5, 4, "Blitz"),
        Rapid10p10 => ("10+10".to_owned(), icon_for_speed(Rapid), 10, 10, "Rapid"),
        Rapid15p10 => ("15+10".to_owned(), icon_for_speed(Rapid), 15, 10, "Rapid"),
        Classic20p20 => (
            "20+20".to_owned(),
            icon_for_speed(Classic),
            20,
            20,
            "Classic",
        ),
        Classic30p30 => (
            "30+30".to_owned(),
            icon_for_speed(Classic),
            30,
            30,
            "Classic",
        ),
    };
    let hover_text = format!("{speed_name}\n{base} min base time\n+{increment} sec per move");
    let mark_pressed = use_tap_feedback(".quickplay-hex-button");
    view! {
        <button
            type="button"
            class="quickplay-hex-button ui-button"
            data-speed=speed_name
            title=hover_text
            on:pointerdown=move |event| mark_pressed.run(event)
            on:click=move |_| {
                if auth_context.user.with(|a| a.is_some()) {
                    let api = api.get();
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

            <Icon icon=icon_data attr:class="quickplay-hex-icon" />
            <span class="quickplay-hex-label">{display_text}</span>
        </button>
    }
}
#[component]
pub fn QuickPlay() -> impl IntoView {
    let i18n = use_i18n();
    let dialog_el = NodeRef::<Dialog>::new();
    let bot_dialog_el = NodeRef::<Dialog>::new();
    let auth_context = expect_context::<AuthContext>();
    let params = expect_context::<Store<ChallengeParams>>();
    let (_, set_cookie) = challenge_params_cookie();
    let mark_custom_pressed = use_tap_feedback(".quickplay-hex-button");
    let mark_bot_pressed = use_tap_feedback(".quickplay-hex-button");
    let _ = use_event_listener(dialog_el, ev::close, move |_| {
        set_cookie.set(Some(params.get()));
    });
    view! {
        <div class="flex flex-col gap-3 items-center py-2 mx-auto w-full max-w-screen-md">
            <Modal dialog_el>
                <ChallengeCreate />
            </Modal>
            <Modal dialog_el=bot_dialog_el>
                <ChallengeBot />
            </Modal>
            <h2 class="text-lg font-bold text-center text-gray-900 dark:text-gray-100">
                {t!(i18n, home.create_game)}
            </h2>
            <div class="quickplay-hex-shell">
                <div class="quickplay-hex-grid" role="group" aria-label="Quick play">
                    <div class="quickplay-hex-cell quickplay-hex-left">
                        <GridButton time_control=Bullet1p2 />
                    </div>
                    <div class="quickplay-hex-cell quickplay-hex-blitz-top">
                        <GridButton time_control=Blitz3p3 />
                    </div>
                    <div class="quickplay-hex-cell quickplay-hex-blitz-bottom">
                        <GridButton time_control=Blitz5p4 />
                    </div>
                    <div class="quickplay-hex-cell quickplay-hex-center">
                        <button
                            type="button"
                            class="quickplay-hex-button quickplay-hex-button-center ui-button"
                            on:pointerdown=move |event| mark_custom_pressed.run(event)
                            on:click=move |_| {
                                if auth_context.user.with(|a| a.is_some()) {
                                    if let Some(dialog_el) = dialog_el.get() {
                                        let _ = dialog_el.show_modal();
                                    }
                                } else {
                                    let navigate = use_navigate();
                                    navigate("/login", Default::default());
                                }
                            }
                        >

                            <span class="quickplay-hex-label">
                                {t!(i18n, home.custom_game.button)}
                            </span>
                        </button>
                    </div>
                    <div class="quickplay-hex-cell quickplay-hex-slow-top">
                        <GridButton time_control=Rapid10p10 />
                    </div>
                    <div class="quickplay-hex-cell quickplay-hex-slow-bottom">
                        <GridButton time_control=Classic20p20 />
                    </div>
                    <div class="quickplay-hex-cell quickplay-hex-right">
                        <button
                            type="button"
                            class="quickplay-hex-button quickplay-hex-button-secondary ui-button"
                            title="Play vs bot"
                            on:pointerdown=move |event| mark_bot_pressed.run(event)
                            on:click=move |_| {
                                if auth_context.user.with(|a| a.is_some()) {
                                    if let Some(dialog_el) = bot_dialog_el.get() {
                                        let _ = dialog_el.show_modal();
                                    }
                                } else {
                                    let navigate = use_navigate();
                                    navigate("/login", Default::default());
                                }
                            }
                        >

                            <Icon
                                icon=icondata_mdi::MdiRobotHappy
                                attr:class="quickplay-hex-icon"
                            />
                            <span class="quickplay-hex-label">"Play bot"</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
