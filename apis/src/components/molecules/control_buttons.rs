use crate::{
    components::atoms::gc_button::{AcceptDenyGc, ConfirmButton},
    providers::{auth_context::AuthContext, game_state::GameStateSignal},
};
use hive_lib::{game_control::GameControl, game_status::GameStatus};
use leptos::*;
use leptos_icons::{
    AiIcon::AiFlagOutlined, AiIcon::AiStopOutlined, BiIcon::BiUndoRegular,
    FaIcon::FaHandshakeSimpleSolid,
};

#[component]
pub fn ControlButtons() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let is_started = move || match (game_state.signal)().state.game_status {
        GameStatus::NotStarted => false,
        _ => true,
    };
    let is_finished = move || match (game_state.signal)().state.game_status {
        GameStatus::Finished(_) => true,
        _ => false,
    };

    let auth_context = expect_context::<AuthContext>();
    let user = move || match untrack(auth_context.user) {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };

    let pending_takeback = move || match (game_state.signal)().game_control_pending {
        Some(GameControl::TakebackRequest(color)) => {
            if let Some(user) = user() {
                if let Some(user_color) = game_state.user_color(user.id) {
                    if color.opposite_color() == user_color {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    };

    let pending_draw = move || match (game_state.signal)().game_control_pending {
        Some(GameControl::DrawOffer(color)) => {
            if let Some(user) = user() {
                if let Some(user_color) = game_state.user_color(user.id) {
                    if color.opposite_color() == user_color {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    };

    // TODO: can we refactor this into a generic send_game_control function?

    let abort = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::Abort(color), user.id);
            }
        }
    });

    let resign = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::Resign(color), user.id);
            }
        }
    });

    let draw_offer = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::DrawOffer(color), user.id);
            }
        }
    });

    let draw_reject = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::DrawReject(color), user.id);
            }
        }
    });

    let draw_accept = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::DrawAccept(color), user.id);
            }
        }
    });

    let takeback_request = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::TakebackRequest(color), user.id);
            }
        }
    });

    let takeback_accept = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::TakebackAccept(color), user.id);
            }
        }
    });

    let takeback_reject = Callback::<()>::from(move |_| {
        if let Some(user) = user() {
            if let Some(color) = game_state.user_color(user.id) {
                game_state.send_game_control(GameControl::TakebackReject(color), user.id);
            }
        }
    });

    view! {
        <Show
            when=is_finished
            fallback=move || {
                view! {
                    <div class="flex justify-around items-center min-w-fit min-h-fit">
                        <Show
                            when=is_started
                            fallback=move || {
                                view! {
                                    <ConfirmButton
                                        icon=leptos_icons::Icon::Ai(AiStopOutlined)
                                        action=abort
                                    />
                                }
                            }
                        >

                            <Show
                                when=pending_takeback
                                fallback=move || {
                                    view! {
                                        <ConfirmButton
                                            icon=leptos_icons::Icon::Bi(BiUndoRegular)
                                            action=takeback_request
                                        />
                                    }
                                }
                            >

                                <div class="relative">
                                    <AcceptDenyGc
                                        icon=leptos_icons::Icon::Bi(BiUndoRegular)
                                        red=false
                                        action=takeback_accept
                                    />
                                    <AcceptDenyGc
                                        icon=leptos_icons::Icon::Bi(BiUndoRegular)
                                        red=true
                                        action=takeback_reject
                                    />
                                </div>
                            </Show>
                        </Show>

                        <Show
                            when=pending_draw
                            fallback=move || {
                                view! {
                                    <ConfirmButton
                                        icon=leptos_icons::Icon::Fa(FaHandshakeSimpleSolid)
                                        action=draw_offer
                                    />
                                }
                            }
                        >

                            <div class="relative">
                                <AcceptDenyGc
                                    icon=leptos_icons::Icon::Fa(FaHandshakeSimpleSolid)
                                    red=false
                                    action=draw_accept
                                />
                                <AcceptDenyGc
                                    icon=leptos_icons::Icon::Fa(FaHandshakeSimpleSolid)
                                    red=true
                                    action=draw_reject
                                />
                            </div>
                        </Show>
                        <ConfirmButton icon=leptos_icons::Icon::Ai(AiFlagOutlined) action=resign/>
                    </div>
                }
            }
        >

            Rematch button/new game button
        </Show>
    }
}
