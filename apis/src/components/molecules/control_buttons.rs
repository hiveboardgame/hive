use crate::{
    common::ChallengeAction,
    components::atoms::gc_button::{AcceptDenyGc, ConfirmButton},
    providers::{
        challenges::ChallengeStateSignal,
        game_state::{GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
    },
};
use hive_lib::{Color, ColorChoice, GameControl};
use leptos::{either::EitherOf3, prelude::*};
use leptos_router::hooks::use_navigate;
use shared_types::{ChallengeDetails, ChallengeVisibility};

#[component]
pub fn ControlButtons() -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let user = auth_context.user;
    let user_id = Signal::derive(move || {
        user.with_untracked(|a| {
            a.as_ref()
                .map(|account| account.id)
                .expect("Control buttons show only for logged in players")
        })
    });
    let is_finished = game_state.is_finished();
    let user_color = game_state.color_for_user_signal(Signal::derive(move || Some(user_id())));
    let color =
        Signal::derive(move || user_color().expect("User_id is one of the players in this game"));
    let pending = Signal::derive(move || game_state.game_control_pending().get());
    let game_response = game_state.game_response();
    let not_tournament = Signal::derive(move || {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(|game| game.tournament.is_none())
        })
    });
    let takeback_allowed = Signal::derive(move || game_state.takeback_allowed());
    let turn = Signal::derive(move || game_state.state().with(|state| state.turn));
    //TODO: Check whether this button works as intended
    let navigate_to_tournament = move |_| {
        let navigate = use_navigate();
        let tournament_id = game_response.with_untracked(|game_response| {
            game_response
                .as_ref()
                .and_then(|game| game.tournament.as_ref())
                .map_or(String::new(), |tournament| {
                    tournament.tournament_id.to_string()
                })
        });
        navigate(&format!("/tournament/{tournament_id}"), Default::default());
    };
    let pending_draw = Signal::derive(move || match pending() {
        Some(GameControl::DrawOffer(gc_color)) => gc_color.opposite_color() == color(),

        _ => false,
    });

    let pending_takeback = move || match pending() {
        Some(GameControl::TakebackRequest(gc_color)) => gc_color.opposite_color() == color(),

        _ => false,
    };

    let new_opponent = move |_| {
        game_response.with_untracked(|game_response| {
            if let Some(game) = game_response.as_ref() {
                let details = ChallengeDetails {
                    rated: game.rated,
                    game_type: game.game_type,
                    visibility: ChallengeVisibility::Public,
                    opponent: None,
                    color_choice: ColorChoice::Random,
                    time_mode: game.time_mode,
                    time_base: game.time_base,
                    time_increment: game.time_increment,
                    band_upper: None,
                    band_lower: None,
                };
                let challenge_action = ChallengeAction::Create(details);
                let api = api.get();
                let navigate = leptos_router::hooks::use_navigate();
                api.challenge(challenge_action);
                navigate("/", Default::default());
            }
        });
    };

    let rematch_present = move || {
        let challenge_state_signal = expect_context::<ChallengeStateSignal>();
        game_response.with(|game_response| {
            let game_response = game_response.as_ref()?;
            challenge_state_signal.signal.with(|cs| {
                cs.challenges
                    .values()
                    .find(|challenge| {
                        challenge.visibility == ChallengeVisibility::Direct
                            && challenge.opponent.clone().is_some_and(|ref opponent| {
                                opponent.uid == game_response.black_player.uid
                                    || opponent.uid == game_response.white_player.uid
                            })
                            && (challenge.challenger.uid == game_response.black_player.uid
                                || challenge.challenger.uid == game_response.white_player.uid)
                            && challenge.game_type == game_response.game_type.to_string()
                            && challenge.time_mode == game_response.time_mode
                            && challenge.time_base == game_response.time_base
                            && challenge.time_increment == game_response.time_increment
                    })
                    .cloned()
            })
        })
    };

    let sent_challenge = move || {
        if let Some(challenge) = rematch_present() {
            return challenge.challenger.uid == user_id();
        }
        false
    };

    let rematch_button_color = move || {
        if let Some(challenge) = rematch_present() {
            if challenge.challenger.uid != user_id() {
                return "bg-grasshopper-green hover:bg-green-500";
            }
        }
        "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
    };

    let rematch_text = move || {
        if let Some(challenge) = rematch_present() {
            if challenge.challenger.uid == user_id() {
                "Sent"
            } else {
                "Accept"
            }
        } else {
            "Rematch"
        }
    };

    let rematch = move |_| {
        if let Some(challenge) = rematch_present() {
            let api = api.get();
            api.challenge_accept(challenge.challenge_id);
        } else if let Some(user_id) = user.with(|a| a.as_ref().map(|u| u.id)) {
            game_response.with_untracked(|game_response| {
                if let Some(game) = game_response.as_ref() {
                    // TODO: color and opponent
                    let (color_choice, opponent) = match game.color_for_user(Some(user_id)) {
                        Some(Color::Black) => {
                            (ColorChoice::White, Some(game.white_player.username.clone()))
                        }
                        Some(Color::White) => {
                            (ColorChoice::Black, Some(game.black_player.username.clone()))
                        }
                        None => unreachable!(),
                    };
                    let details = ChallengeDetails {
                        rated: game.rated,
                        game_type: game.game_type,
                        visibility: ChallengeVisibility::Direct,
                        opponent,
                        color_choice,
                        time_mode: game.time_mode,
                        time_base: game.time_base,
                        time_increment: game.time_increment,
                        band_upper: None,
                        band_lower: None,
                    };
                    let challenge_action = ChallengeAction::Create(details);
                    let api = api.get();
                    api.challenge(challenge_action);
                }
            });
        }
    };
    move || {
        if is_finished() {
            if not_tournament() {
                EitherOf3::A(view! {
                    <button
                        class=move || {
                            format!(
                                "h-7 m-1 grow {} transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-2 rounded disabled:opacity-25 disabled:cursor-not-allowed flex-shrink-0",
                                rematch_button_color(),
                            )
                        }

                        prop:disabled=sent_challenge
                        on:click=rematch
                    >
                        {rematch_text}
                    </button>
                    <button
                        class="flex-shrink-0 py-1 px-2 m-1 h-7 font-bold text-white rounded transition-transform duration-300 active:scale-95 grow bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal"
                        on:click=new_opponent
                    >
                        New Game
                    </button>
                })
            } else {
                EitherOf3::B(view! {
                    <button
                        class="flex-shrink-0 py-1 px-2 m-1 h-7 font-bold text-white rounded transition-transform duration-300 active:scale-95 grow bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal"
                        on:click=navigate_to_tournament
                    >
                        View tournament
                    </button>
                })
            }
        } else {
            EitherOf3::C(view! {
                <div class="flex flex-col w-full">
                    <div class="flex justify-around items-center pt-1 grow shrink">
                        <Show when=not_tournament>
                            <div class="relative">
                                <ConfirmButton
                                    game_control=GameControl::Abort(color())
                                    user_id=user_id()
                                    hidden=Signal::derive(move || turn() > 1)
                                />
                                <Show when=takeback_allowed>
                                    <ConfirmButton
                                        game_control=GameControl::TakebackRequest(color())
                                        user_id=user_id()
                                        hidden=Signal::derive(move || {
                                            pending_takeback() || turn() < 2
                                        })
                                    />

                                    <AcceptDenyGc
                                        game_control=GameControl::TakebackAccept(color())

                                        user_id=user_id()
                                        hidden=Signal::derive(move || !pending_takeback())
                                    />
                                    <AcceptDenyGc
                                        game_control=GameControl::TakebackReject(color())
                                        user_id=user_id()
                                        hidden=Signal::derive(move || !pending_takeback())
                                    />
                                </Show>
                            </div>
                        </Show>
                        <div class="relative">
                            <ConfirmButton
                                game_control=GameControl::DrawOffer(color())
                                user_id=user_id()
                                hidden=pending_draw
                            />

                            <AcceptDenyGc
                                game_control=GameControl::DrawAccept(color())
                                user_id=user_id()
                                hidden=Signal::derive(move || !pending_draw())
                            />
                            <AcceptDenyGc
                                game_control=GameControl::DrawReject(color())
                                user_id=user_id()
                                hidden=Signal::derive(move || !pending_draw())
                            />
                        </div>
                        <ConfirmButton
                            game_control=GameControl::Resign(color())
                            user_id=user_id()
                        />
                    </div>

                    <div class="flex justify-center w-full h-5">
                        <Show when=pending_takeback>
                            <span class="font-bold">"Opponent wants a takeback"</span>
                        </Show>
                        <Show when=pending_draw>
                            <span class="font-bold">"Opponent offers a draw"</span>
                        </Show>
                    </div>
                </div>
            })
        }
    }
}
