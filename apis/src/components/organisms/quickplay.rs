use crate::{
    common::ChallengeAction,
    components::{atoms::rating::icon_for_speed, molecules::modal::Modal},
    functions::auth::guest::guest_login,
    i18n::*,
    pages::{challenge_bot::ChallengeBot, challenge_create::ChallengeCreate},
    providers::{challenge_params_cookie, ApiRequestsProvider, AuthContext, ChallengeParams},
};
use hive_lib::{ColorChoice, GameType};
use leptos::{ev, html::Dialog, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use leptos_use::use_event_listener;
use reactive_stores::Store;
use shared_types::{ChallengeDetails, ChallengeVisibility, GameSpeed::*, TimeMode};

/// Which dialog to open once a guest session has been provisioned.
#[derive(Clone, Copy)]
enum GuestIntent {
    Bot,
    Friend,
}

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
const BUTTON_STYLE: &str = "flex w-full gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95";

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
    view! {
        <button
            class=BUTTON_STYLE
            title=hover_text
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

            <Icon icon=icon_data />
            {display_text}
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
    let _ = use_event_listener(dialog_el, ev::close, move |_| {
        set_cookie.set(Some(params.get()));
    });

    // Full account = logged in and not a guest. Guests and logged-out visitors
    // see the casual guest CTA instead of the rated time-control grid.
    let user = auth_context.user;
    let is_full_user = move || user.with(|a| a.as_ref().is_some_and(|account| !account.user.guest));

    // Provision a guest, then open the requested dialog. When already signed in
    // (guest or full) we skip straight to opening it.
    let guest_action = Action::new(|_: &()| async { guest_login().await });
    let pending = RwSignal::new(None::<GuestIntent>);
    let open_intent = move |intent: GuestIntent| match intent {
        GuestIntent::Bot => {
            if let Some(d) = bot_dialog_el.get() {
                let _ = d.show_modal();
            }
        }
        GuestIntent::Friend => {
            if let Some(d) = dialog_el.get() {
                let _ = d.show_modal();
            }
        }
    };
    let play = move |intent: GuestIntent| {
        if user.with(|a| a.is_some()) {
            open_intent(intent);
        } else {
            pending.set(Some(intent));
            guest_action.dispatch(());
        }
    };
    Effect::watch(
        move || guest_action.value().get(),
        move |val, _, _| {
            if let Some(Ok(_)) = val {
                // Reconnect the websocket as the new guest before they submit
                // the challenge from the now-open dialog.
                auth_context.refresh(true);
                if let Some(intent) = pending.get_untracked() {
                    open_intent(intent);
                    pending.set(None);
                }
            }
        },
        false,
    );

    view! {
        <div class="flex flex-col items-center m-2 grow">
            <Modal dialog_el>
                <ChallengeCreate />
            </Modal>
            <Modal dialog_el=bot_dialog_el>
                <ChallengeBot />
            </Modal>
            <Show
                when=is_full_user
                fallback=move || {
                    view! {
                        <span class="mb-1 text-xl font-bold">"Play your first game"</span>
                        <span class="mb-4 text-sm text-center opacity-80">
                            "No signup needed — register later to keep your games"
                        </span>
                        <div class="flex flex-col gap-2 w-full max-w-xs sm:gap-4">
                            <button
                                class=BUTTON_STYLE
                                title="Play vs a bot — no account needed"
                                on:click=move |_| play(GuestIntent::Bot)
                            >
                                "Play your first game"
                            </button>
                            <button
                                class=BUTTON_STYLE
                                title="Create a link to share with a friend"
                                on:click=move |_| play(GuestIntent::Friend)
                            >
                                "Play a friend"
                            </button>
                            <button
                                class=BUTTON_STYLE
                                title="Two players, one device"
                                on:click=move |_| {
                                    let navigate = use_navigate();
                                    navigate("/analysis", Default::default());
                                }
                            >
                                "Pass & play"
                            </button>
                        </div>
                    }
                }
            >
                <span class="flex justify-center mb-4 text-xl font-bold">
                    {t!(i18n, home.create_game)}
                </span>
                <div class="grid grid-cols-3 gap-2 w-full sm:gap-4">
                    <GridButton time_control=Bullet1p2 />
                    <GridButton time_control=Blitz3p3 />
                    <GridButton time_control=Blitz5p4 />
                    <GridButton time_control=Rapid10p10 />
                    <GridButton time_control=Classic20p20 />
                    <button
                        class=BUTTON_STYLE
                        on:click=move |_| {
                            if let Some(dialog_el) = dialog_el.get() {
                                let _ = dialog_el.show_modal();
                            }
                        }
                    >

                        {t!(i18n, home.custom_game.button)}
                    </button>
                    <button
                        class=format!("{} col-start-2", BUTTON_STYLE)
                        title="Play vs bot"
                        on:click=move |_| {
                            if let Some(dialog_el) = bot_dialog_el.get() {
                                let _ = dialog_el.show_modal();
                            }
                        }
                    >

                        "Play bot"
                    </button>
                </div>
            </Show>
        </div>
    }
}
