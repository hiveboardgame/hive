use crate::providers::game_state::GameStateSignal;
use hive_lib::GameControl;
use icondata_core;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use uuid::Uuid;

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
            "bg-red-700 hover:bg-ladybug-red absolute"
        }
        _ => "mr-1 bg-grasshopper-green hover:bg-green-500 relative",
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
                format!(
                    "aspect-square rounded-sm transform transition-transform duration-300 active:scale-95 [&>svg]:size-6 lg:[&>svg]:size-8 {} {}",
                    button_style(),
                    hidden,
                )
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
    let pending_slice = create_read_slice(game_state.signal, |gs| gs.game_control_pending.clone());
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
        let base = String::from("aspect-square rounded-sm relative transform transition-transform duration-300 active:scale-95 [&>svg]:size-6 lg:[&>svg]:size-8 ");
        base + if is_clicked() {
            "bg-grasshopper-green hover:bg-green-500"
        } else if pending(game_control) {
            "bg-pillbug-teal text-slate-500"
        } else if disabled() {
            "text-slate-500"
        } else {
            "hover:bg-grasshopper-green"
        }
    };

    view! {
        <div class=move || { if hidden() { "relative hidden" } else { "relative" } }>
            <button title=title on:click=onclick_confirm disabled=disabled class=class>
                <Icon icon />
            </button>
            <Show when=is_clicked>
                <button
                    title="Cancel"
                    on:click=cancel
                    class="absolute ml-1 bg-red-700 rounded-sm duration-300 aspect-square hover:bg-ladybug-red"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6 lg:size-8" />
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
