use crate::{
    common::ChallengeAction,
    components::atoms::gc_button::{AcceptDenyGc, ConfirmButton},
    providers::{
        challenges::ChallengeStateSignal,
        game_state::{GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
    },
};
use hive_lib::{ColorChoice, GameControl};
use leptos::{either::EitherOf3, prelude::*};
use leptos_router::hooks::use_navigate;
use shared_types::{ChallengeDetails, ChallengeVisibility};

const FINISHED_GAME_BUTTON_CLASS: &str =
    "ui-button m-1 h-8 min-h-8 grow rounded px-2 py-1 leading-none";

#[component]
pub fn ControlButtons() -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let identity = auth_context.identity;
    let user_id = move || {
        identity
            .get_untracked()
            .and_then(AuthIdentity::user_id)
            .expect("Control buttons show only for logged in players")
    };
    let is_finished = game_state.is_finished();
    let user_color = game_state.user_color_as_signal(identity);
    let color = Memo::new(move |_| {
        user_color
            .get()
            .expect("User_id is one of the players in this game")
    });
    let pending = game_state.game_control_pending();
    let game_response = game_state.game_response();
    let state = game_state.state();
    let turn = Memo::new(move |_| state.with(|state| state.turn));
    let not_tournament = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(|gr| gr.tournament.is_none())
        })
    });
    let takeback_allowed = Memo::new(move |_| game_state.takeback_allowed());
    //TODO: Check whether this button works as intended
    let navigate_to_tournament = move |_| {
        let navigate = use_navigate();
        navigate(
            &format!(
                "/tournament/{}",
                game_response.with(|game_response| {
                    game_response.as_ref().map_or(String::new(), |gr| {
                        gr.tournament
                            .as_ref()
                            .map_or(String::new(), |t| t.tournament_id.to_string())
                    })
                })
            ),
            Default::default(),
        );
    };
    let pending_draw = Signal::derive(move || match pending.get() {
        Some(GameControl::DrawOffer(gc_color)) => gc_color.opposite_color() == color(),

        _ => false,
    });

    let pending_takeback = move || match pending.get() {
        Some(GameControl::TakebackRequest(gc_color)) => gc_color.opposite_color() == color(),

        _ => false,
    };

    let new_opponent = move |_| {
        let Some(details) = game_response.with_untracked(|game| {
            let game = game.as_ref()?;
            Some(ChallengeDetails {
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
            })
        }) else {
            return;
        };
        let challenge_action = ChallengeAction::Create(details);
        let api = api.get();
        let navigate = leptos_router::hooks::use_navigate();
        api.challenge(challenge_action);
        navigate("/", Default::default());
    };

    let challenge_state = expect_context::<ChallengeStateSignal>();
    let rematch_present = Memo::new(move |_| {
        game_response.with(|game_response| {
            let game_response = game_response.as_ref()?;
            let game_type = game_response.game_type.to_string();
            challenge_state.signal.with(|state| {
                state
                    .challenges
                    .values()
                    .find(|challenge| {
                        challenge.visibility == ChallengeVisibility::Direct
                            && challenge.opponent.as_ref().is_some_and(|opponent| {
                                opponent.uid == game_response.black_player.uid
                                    || opponent.uid == game_response.white_player.uid
                            })
                            && (challenge.challenger.uid == game_response.black_player.uid
                                || challenge.challenger.uid == game_response.white_player.uid)
                            && challenge.game_type == game_type
                            && challenge.time_mode == game_response.time_mode
                            && challenge.time_base == game_response.time_base
                            && challenge.time_increment == game_response.time_increment
                    })
                    .map(|challenge| (challenge.challenge_id.clone(), challenge.challenger.uid))
            })
        })
    });

    let sent_challenge = move || {
        rematch_present.with(|challenge| {
            challenge
                .as_ref()
                .is_some_and(|(_, challenger_id)| *challenger_id == user_id())
        })
    };

    let rematch_button_tone = move || {
        rematch_present.with(|challenge| {
            if challenge
                .as_ref()
                .is_some_and(|(_, challenger_id)| *challenger_id != user_id())
            {
                "ui-button-success"
            } else {
                "ui-button-primary"
            }
        })
    };

    let rematch_text = move || {
        rematch_present.with(|challenge| {
            if let Some((_, challenger_id)) = challenge {
                if *challenger_id == user_id() {
                    "Sent"
                } else {
                    "Accept"
                }
            } else {
                "Rematch"
            }
        })
    };

    let rematch = move |_| {
        if let Some(challenge_id) =
            rematch_present.with_untracked(|challenge| challenge.as_ref().map(|(id, _)| id.clone()))
        {
            let api = api.get();
            api.challenge_accept(challenge_id);
        } else if let Some(user_id) = identity.get_untracked().and_then(AuthIdentity::user_id) {
            if let Some(details) = game_response.with_untracked(|game| {
                let game = game.as_ref()?;
                // TODO: color and opponent
                let (color_choice, opponent) = if user_id == game.black_player.uid {
                    (ColorChoice::White, Some(game.white_player.username.clone()))
                } else if user_id == game.white_player.uid {
                    (ColorChoice::Black, Some(game.black_player.username.clone()))
                } else {
                    unreachable!();
                };
                Some(ChallengeDetails {
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
                })
            }) {
                let challenge_action = ChallengeAction::Create(details);
                let api = api.get();
                api.challenge(challenge_action);
            }
        }
    };
    move || {
        if is_finished() {
            if not_tournament() {
                EitherOf3::A(view! {
                    <button
                        class=move || {
                            format!("{} {}", FINISHED_GAME_BUTTON_CLASS, rematch_button_tone())
                        }

                        prop:disabled=sent_challenge
                        on:click=rematch
                    >
                        {rematch_text}
                    </button>
                    <button
                        class=format!("{FINISHED_GAME_BUTTON_CLASS} ui-button-primary")
                        on:click=new_opponent
                    >
                        New Game
                    </button>
                })
            } else {
                EitherOf3::B(view! {
                    <button
                        class=format!("{FINISHED_GAME_BUTTON_CLASS} ui-button-primary")
                        on:click=navigate_to_tournament
                    >
                        View tournament
                    </button>
                })
            }
        } else {
            EitherOf3::C(view! {
                <div class="flex flex-col w-full">
                    <div class="flex justify-around items-center grow shrink">
                        <Show when=not_tournament>
                            <div class="flex relative items-center">
                                <ConfirmButton
                                    game_control=GameControl::Abort(color())
                                    user_id=user_id()

                                    hidden=Signal::derive(move || { turn() > 1 })
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
                        <div class="flex relative items-center">
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
