use crate::{common::with_class, providers::game_state::GameStateSignal};
use hudsoni::GameControl;
use icondata_core;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use uuid::Uuid;

const GAME_CONTROL_ICON_BASE_CLASS: &str = "ui-game-control-button";
const GAME_CONTROL_ACCEPT_CLASS: &str = "bg-grasshopper-green text-white hover:bg-green-500";

#[component]
pub fn AcceptDenyGc(
    game_control: GameControl,
    user_id: Uuid,
    #[prop(optional, into)] hidden: Signal<bool>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let (icon, title) = get_icon_and_title(game_control);

    let button_style = move || match game_control {
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {
            "bg-ladybug-red text-white hover:bg-red-500".to_string()
        }
        _ => with_class(GAME_CONTROL_ACCEPT_CLASS, "relative"),
    };

    let on_click = move |_| {
        game_state.send_game_control(game_control, user_id);
    };
    view! {
        <button
            title=title
            on:click=on_click
            class=move || {
                let hidden = if hidden() { "hidden" } else { "" };
                with_class(GAME_CONTROL_ICON_BASE_CLASS, format!("{} {}", button_style(), hidden))
            }
        >

            <Icon icon />
        </button>
    }
}

#[component]
pub fn ConfirmButton(
    game_control: GameControl,
    user_id: Uuid,
    #[prop(optional, into)] hidden: Signal<bool>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let pending_slice = create_read_slice(game_state.signal, |gs| gs.game_control_pending);
    let turn = create_read_slice(game_state.signal, |gs| gs.state.turn as i32);
    let (icon, title) = get_icon_and_title(game_control);
    let color = game_control.color();
    let is_clicked = RwSignal::new(false);
    let UseTimeoutFnReturn { start, stop, .. } = use_timeout_fn(
        move |_: ()| {
            is_clicked.update(|v| *v = false);
        },
        5000.0,
    );
    let stop = StoredValue::new(stop);
    let onclick_confirm = move |_| {
        if is_clicked() {
            (stop.get_value())();
            game_state.send_game_control(game_control, user_id);
            is_clicked.update(|v| *v = false);
        } else {
            is_clicked.set(true);
            start(());
        }
    };

    let cancel = move |_| {
        (stop.get_value())();
        is_clicked.set(false);
    };
    let pending = move |game_control: GameControl| match pending_slice() {
        Some(GameControl::DrawOffer(gc_color)) => {
            if color == gc_color && matches!(game_control, GameControl::DrawOffer(_)) {
                return true;
            }
            false
        }
        Some(GameControl::TakebackRequest(gc_color)) => {
            if color == gc_color && matches!(game_control, GameControl::TakebackRequest(_)) {
                return true;
            }
            false
        }
        _ => false,
    };

    let disabled = move || {
        let game_control = game_control;
        if game_control.allowed_on_turn(turn()) {
            !matches!(game_control, GameControl::Resign(_))
                && (pending(GameControl::DrawOffer(color))
                    || pending(GameControl::TakebackRequest(color)))
        } else {
            true
        }
    };

    let class = move || {
        let state = if is_clicked() {
            GAME_CONTROL_ACCEPT_CLASS
        } else if pending(game_control) {
            "bg-pillbug-teal text-white"
        } else if disabled() {
            "text-gray-500 dark:text-gray-400"
        } else {
            "text-gray-800 hover:bg-grasshopper-green hover:text-white dark:text-gray-100"
        };
        with_class(GAME_CONTROL_ICON_BASE_CLASS, format!("relative {state}"))
    };

    view! {
        <div class=move || {
            if hidden() { "relative hidden" } else { "relative inline-flex items-center gap-1" }
        }>
            <button title=title on:click=onclick_confirm disabled=disabled class=class>
                <Icon icon />
            </button>
            <Show when=is_clicked>
                <button
                    title="Cancel"
                    on:click=cancel
                    class="ui-game-control-button ui-button-danger"
                >
                    <Icon icon=icondata_io::IoCloseSharp />
                </button>
            </Show>
        </div>
    }
}

fn get_icon_and_title(
    game_control: GameControl,
) -> (&'static icondata_core::IconData, &'static str) {
    match game_control {
        GameControl::Abort(_) => (icondata_ai::AiStopOutlined, "Abort"),
        GameControl::DrawAccept(_) => (icondata_fa::FaHandshakeSimpleSolid, "Accept Draw"),
        GameControl::DrawOffer(_) => (icondata_fa::FaHandshakeSimpleSolid, "Offer Draw"),
        GameControl::DrawReject(_) => (icondata_fa::FaHandshakeSimpleSolid, "Reject Draw"),
        GameControl::Resign(_) => (icondata_ai::AiFlagOutlined, "Resign"),
        GameControl::TakebackAccept(_) => (icondata_bi::BiUndoRegular, "Accept Takeback"),
        GameControl::TakebackReject(_) => (icondata_bi::BiUndoRegular, "Reject Takeback"),
        GameControl::TakebackRequest(_) => (icondata_bi::BiUndoRegular, "Request Takeback"),
    }
}
