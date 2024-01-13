use crate::{
    common::challenge_action::{ChallengeAction, ChallengeVisibility},
    components::atoms::gc_button::{AcceptDenyGc, ConfirmButton},
    providers::{
        api_requests::ApiRequests, auth_context::AuthContext, game_state::GameStateSignal,
    },
};
use hive_lib::{
    color::{Color, ColorChoice},
    game_control::GameControl,
    game_status::GameStatus,
};
use leptos::*;

#[component]
pub fn ControlButtons() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();

    let is_finished = move || {
        matches!(
            (game_state.signal)().state.game_status,
            GameStatus::Finished(_)
        )
    };

    let auth_context = expect_context::<AuthContext>();
    let user_id = match untrack(auth_context.user) {
        Some(Ok(Some(user))) => Some(user.id),
        _ => None,
    }
    .expect("User is some");
    let color = game_state
        .user_color(user_id)
        .expect("User is either white or black");

    let abort_allowed = move || {
        let state = (game_state.signal)().state;
        state.turn == 0 || (state.turn == 1 && color == Color::Black)
    };

    let pending_takeback = move || match (game_state.signal)().game_control_pending {
        Some(GameControl::TakebackRequest(gc_color)) => gc_color.opposite_color() == color,

        _ => false,
    };

    let pending_draw = move || match (game_state.signal)().game_control_pending {
        Some(GameControl::DrawOffer(gc_color)) => gc_color.opposite_color() == color,

        _ => false,
    };

    let rematch = move |_| {
        let game_state = expect_context::<GameStateSignal>();
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(user))) = untrack(auth_context.user) {
            if let Some(game) = game_state.signal.get_untracked().game_response {
                // TODO: color and opponent
                let (color_choice, opponent) = if user.id == game.black_player.uid {
                    (ColorChoice::White, Some(game.white_player.username))
                } else if user.id == game.white_player.uid {
                    (ColorChoice::Black, Some(game.black_player.username))
                } else {
                    unreachable!();
                };
                let challenge_action = ChallengeAction::Create {
                    rated: game.rated,
                    game_type: game.game_type,
                    visibility: ChallengeVisibility::Direct,
                    opponent,
                    color_choice,
                    time_mode: game.time_mode,
                    time_base: game.time_base,
                    time_increment: game.time_increment,
                };
                let api = ApiRequests::new();
                api.challenge_direct(challenge_action);
            }
        }
    };

    view! {
        <div class="flex justify-around items-center grow shrink w-full">
            <Show
                when=is_finished
                fallback=move || {
                    view! {
                        <div class="flex justify-around items-center grow shrink">
                            <Show
                                when=abort_allowed
                                fallback=move || {
                                    view! {
                                        <Show
                                            when=pending_takeback
                                            fallback=move || {
                                                view! {
                                                    <ConfirmButton
                                                        game_control=store_value(
                                                            GameControl::TakebackRequest(color),
                                                        )

                                                        user_id=user_id
                                                    />
                                                }
                                            }
                                        >

                                            <div class="relative">
                                                <AcceptDenyGc
                                                    game_control=store_value(GameControl::TakebackAccept(color))
                                                    user_id=user_id
                                                />
                                                <AcceptDenyGc
                                                    game_control=store_value(GameControl::TakebackReject(color))
                                                    user_id=user_id
                                                />
                                            </div>
                                        </Show>
                                    }
                                }
                            >

                                <Show
                                    when=pending_takeback
                                    fallback=move || {
                                        view! {
                                            <ConfirmButton
                                                game_control=store_value(GameControl::Abort(color))
                                                user_id=user_id
                                            />
                                        }
                                    }
                                >

                                    <div class="relative">
                                        <AcceptDenyGc
                                            game_control=store_value(GameControl::TakebackAccept(color))
                                            user_id=user_id
                                        />
                                        <AcceptDenyGc
                                            game_control=store_value(GameControl::TakebackReject(color))
                                            user_id=user_id
                                        />
                                    </div>
                                </Show>

                            </Show>

                            <Show
                                when=pending_draw
                                fallback=move || {
                                    view! {
                                        <ConfirmButton
                                            game_control=store_value(GameControl::DrawOffer(color))
                                            user_id=user_id
                                        />
                                    }
                                }
                            >

                                <div class="relative">
                                    <AcceptDenyGc
                                        game_control=store_value(GameControl::DrawAccept(color))
                                        user_id=user_id
                                    />
                                    <AcceptDenyGc
                                        game_control=store_value(GameControl::DrawReject(color))
                                        user_id=user_id
                                    />
                                </div>
                            </Show>
                            <ConfirmButton
                                game_control=store_value(GameControl::Resign(color))
                                user_id=user_id
                            />
                        </div>
                    }
                }
            >

                <button
                    class="m-1 grow md:grow-0 whitespace-nowrap bg-blue-500 hover:bg-blue-700 duration-300 text-white font-bold py-2 px-4 rounded"
                    on:click=rematch
                >
                    Rematch
                </button>
                <button class="m-1 grow md:grow-0 whitespace-nowrap bg-blue-500 hover:bg-blue-700 duration-300 text-white font-bold py-2 px-4 rounded">
                    New game
                </button>
            </Show>
        </div>
    }
}
