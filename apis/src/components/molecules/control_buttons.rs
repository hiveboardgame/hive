use crate::{
    common::ChallengeAction,
    components::atoms::gc_button::{AcceptDenyGc, ConfirmButton},
    providers::{
        challenges::ChallengeStateSignal, game_state::GameStateSignal, ApiRequestsProvider,
        AuthContext,
    },
};
use hive_lib::{ColorChoice, GameControl};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use shared_types::{ChallengeDetails, ChallengeVisibility};

#[component]
pub fn ControlButtons() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let user_id = move || {
        auth_context
            .user
            .get_untracked()
            .and_then(|result| result.ok())
            .map(|account| account.id)
            .expect("Control buttons show only for logged in players")
    };

    let color = Signal::derive(move || {
        game_state
            .user_color_as_signal(Some(user_id()).into())
            .get()
            .expect("User_id is one of the players in this game")
    });
    let pending = create_read_slice(game_state.signal, |gs| gs.game_control_pending.clone());
    let not_tournament = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .is_some_and(|gr| gr.tournament.is_none())
    });
    let takeback_allowed = create_read_slice(game_state.signal, |gs| gs.takeback_allowed());
    //TODO: Check whether this button works as intended
    let navigate_to_tournament = move |_| {
        let navigate = use_navigate();
        navigate(
            &format!(
                "/tournament/{}",
                game_state
                    .signal
                    .get()
                    .game_response
                    .as_ref()
                    .map_or(String::new(), |gr| gr
                        .tournament
                        .as_ref()
                        .map_or(String::new(), |t| t.tournament_id.to_string()))
            ),
            Default::default(),
        );
    };
    let pending_draw = move || match pending() {
        Some(GameControl::DrawOffer(gc_color)) => gc_color.opposite_color() == color(),

        _ => false,
    };

    let pending_takeback = move || match pending() {
        Some(GameControl::TakebackRequest(gc_color)) => gc_color.opposite_color() == color(),

        _ => false,
    };

    let new_opponent = move |_| {
        if let Some(game) = game_state.signal.get_untracked().game_response {
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
    };

    let rematch_present = move || {
        let challenge_state_signal = expect_context::<ChallengeStateSignal>();
        let game_state = game_state.signal.get();
        if let Some(game_response) = game_state.game_response {
            challenge_state_signal
                .signal
                .get()
                .challenges
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
        } else {
            None
        }
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
        "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
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
        } else if let Some(Ok(user)) = auth_context.user.get() {
            if let Some(game) = game_state.signal.get_untracked().game_response {
                // TODO: color and opponent
                let (color_choice, opponent) = if user.id == game.black_player.uid {
                    (ColorChoice::White, Some(game.white_player.username))
                } else if user.id == game.white_player.uid {
                    (ColorChoice::Black, Some(game.black_player.username))
                } else {
                    unreachable!();
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
        }
    };

    view! {
        <div class="flex justify-around items-center w-full grow shrink">
            <Show
                when=game_state.is_finished()
                fallback=move || {
                    view! {
                        <div class="flex flex-col w-full">
                            <div class="flex justify-around items-center grow shrink">
                                <Show when=not_tournament>
                                    <div class="relative">
                                        <ConfirmButton
                                            game_control=StoredValue::new(GameControl::Abort(color()))
                                            user_id=user_id()
                                            hidden=memo_for_hidden_class(move || {
                                                (game_state.signal)().state.turn > 1
                                            })
                                        />
                                        <Show when=takeback_allowed>
                                            <ConfirmButton
                                                game_control=StoredValue::new(
                                                    GameControl::TakebackRequest(color()),
                                                )

                                                user_id=user_id()
                                                hidden=memo_for_hidden_class(move || {
                                                    pending_takeback() || (game_state.signal)().state.turn < 2
                                                })
                                            />

                                            <AcceptDenyGc
                                                game_control=StoredValue::new(
                                                    GameControl::TakebackAccept(color()),
                                                )

                                                user_id=user_id()
                                                hidden=memo_for_hidden_class(move || !pending_takeback())
                                            />
                                            <AcceptDenyGc
                                                game_control=StoredValue::new(
                                                    GameControl::TakebackReject(color()),
                                                )

                                                user_id=user_id()
                                                hidden=memo_for_hidden_class(move || !pending_takeback())
                                            />
                                        </Show>
                                    </div>
                                </Show>
                                <div class="relative">
                                    <ConfirmButton
                                        game_control=StoredValue::new(GameControl::DrawOffer(color()))
                                        user_id=user_id()
                                        hidden=memo_for_hidden_class(pending_draw)
                                    />

                                    <AcceptDenyGc
                                        game_control=StoredValue::new(GameControl::DrawAccept(color()))
                                        user_id=user_id()
                                        hidden=memo_for_hidden_class(move || !pending_draw())
                                    />
                                    <AcceptDenyGc
                                        game_control=StoredValue::new(GameControl::DrawReject(color()))
                                        user_id=user_id()
                                        hidden=memo_for_hidden_class(move || !pending_draw())
                                    />
                                </div>
                                <ConfirmButton
                                    game_control=StoredValue::new(GameControl::Resign(color()))
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
                    }
                }
            >

                <Show
                    when=not_tournament
                    fallback=move || {
                        view! {
                            <button
                                class="flex-shrink-0 px-2 py-1 m-1 h-7 font-bold text-white rounded transition-transform duration-300 transform grow bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
                                on:click=navigate_to_tournament
                            >
                                View tournament
                            </button>
                        }
                    }
                >

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
                        class="flex-shrink-0 px-2 py-1 m-1 h-7 font-bold text-white rounded transition-transform duration-300 transform grow bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
                        on:click=new_opponent
                    >
                        New Game
                    </button>
                </Show>
            </Show>
        </div>
    }
}

fn memo_for_hidden_class(condition: impl Fn() -> bool + Send + Sync + 'static) -> Memo<String> {
    Memo::new(move |_| {
        if condition() {
            String::from("hidden")
        } else {
            String::new()
        }
    })
}
