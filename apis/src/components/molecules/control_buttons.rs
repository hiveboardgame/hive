use crate::{
    common::{ChallengeAction, GameAction},
    components::atoms::gc_button::{AcceptDenyGc, ConfirmButton},
    i18n::*,
    providers::{
        challenges::ChallengeStateSignal,
        game_state::{GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
    },
};
use hive_lib::{Color, ColorChoice, GameControl, GameStatus};
use leptos::{either::EitherOf3, prelude::*};
use leptos_icons::Icon;
use leptos_router::hooks::use_navigate;
use shared_types::{tournament::Format, ChallengeDetails, ChallengeVisibility, GameStart};

const FINISHED_GAME_BUTTON_CLASS: &str =
    "ui-button m-1 h-8 min-h-8 grow rounded px-2 py-1 leading-none";

fn arena_berserk_is_available(finished: bool, turn: usize, color: Color, berserked: bool) -> bool {
    let opening_turn = match color {
        Color::White => turn == 0,
        Color::Black => turn <= 1,
    };
    !finished && opening_turn && !berserked
}

#[component]
pub fn ControlButtons() -> impl IntoView {
    let i18n = use_i18n();
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
    let can_berserk = Memo::new(move |_| {
        game_response.with(|response| {
            let Some(game) = response.as_ref() else {
                return false;
            };
            let is_arena = game
                .tournament
                .as_ref()
                .is_some_and(|tournament| tournament.format() == Format::Arena);
            if !is_arena {
                return false;
            }
            arena_berserk_is_available(
                game.finished,
                game.turn,
                color.get(),
                match color.get() {
                    Color::White => game.white_berserked,
                    Color::Black => game.black_berserked,
                },
            )
        })
    });
    let berserk = move |_| {
        if !can_berserk.get_untracked() {
            return;
        }
        if let Some(game_id) = game_response
            .with_untracked(|response| response.as_ref().map(|game| game.game_id.clone()))
        {
            api.get().game(game_id, GameAction::Berserk);
        }
    };
    let not_tournament = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(|gr| gr.tournament.is_none())
        })
    });
    let resumed_move_opening = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response.as_ref().is_some_and(|game| {
                game.tournament.is_none()
                    && game.game_start == GameStart::Moves
                    && game.game_status == GameStatus::InProgress
                    && game.turn < 2
            })
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
                        <Show when=can_berserk>
                            <button
                                on:click=berserk
                                title=move || {
                                    t_string!(i18n, game.arena.berserk_title).to_string()
                                }
                                aria-label=move || {
                                    t_string!(i18n, game.arena.berserk).to_string()
                                }
                                class="ui-button ui-button-danger ui-button-md !px-3"
                            >
                                <Icon icon=icondata_bs::BsLightningFill attr:class="size-5" />
                            </button>
                        </Show>
                        <Show when=not_tournament>
                            <div class="flex relative items-center">
                                <ConfirmButton
                                    game_control=GameControl::Abort(color())
                                    user_id=user_id()

                                    hidden=Signal::derive(move || {
                                        turn() > 1 || resumed_move_opening.get()
                                    })
                                />
                                <Show when=takeback_allowed>
                                    <ConfirmButton
                                        game_control=GameControl::TakebackRequest(color())
                                        user_id=user_id()
                                        hidden=Signal::derive(move || {
                                            pending_takeback() || turn() == 0
                                                || (turn() < 2 && !resumed_move_opening.get())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_gets_one_more_opening_turn_than_white() {
        assert!(!arena_berserk_is_available(false, 1, Color::White, false,));
        assert!(arena_berserk_is_available(false, 1, Color::Black, false,));
    }
}
