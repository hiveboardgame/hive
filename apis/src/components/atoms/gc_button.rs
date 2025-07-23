use crate::providers::game_state::GameStateSignal;
use hive_lib::GameControl;
use icondata_core;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_interval_with_options, UseIntervalOptions};
use std::sync::Arc;
use uuid::Uuid;

#[component]
pub fn AcceptDenyGc(
    game_control: StoredValue<GameControl>,
    user_id: Uuid,
    #[prop(optional, into)] hidden: Signal<String>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let (icon, title) = get_icon_and_title(game_control.get_value());

    let button_style = move || match game_control.get_value() {
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {
            "bg-red-700 hover:bg-ladybug-red absolute"
        }
        _ => "mr-1 bg-grasshopper-green hover:bg-green-500 relative",
    };

    let on_click = move |_| {
        game_state.send_game_control(game_control.get_value(), user_id);
    };
    view! {
        <button
            title=title
            on:click=on_click
            class=move || {
                format!(
                    "aspect-square rounded-sm transform transition-transform duration-300 active:scale-95 {} {}",
                    button_style(),
                    hidden(),
                )
            }
        >

            <Icon icon=icon attr:class="w-6 h-6 lg:h-8 lg:w-8" />
        </button>
    }
}

#[component]
pub fn ConfirmButton(
    game_control: StoredValue<GameControl>,
    user_id: Uuid,
    #[prop(optional, into)] hidden: Signal<String>,
) -> impl IntoView {
    let game_state = StoredValue::new(expect_context::<GameStateSignal>());
    let (icon, title) = get_icon_and_title(game_control.get_value());
    let color = game_control.with_value(|g| g.color());
    let is_clicked = RwSignal::new(false);
    let interval = StoredValue::new(Arc::new(use_interval_with_options(
        5000,
        UseIntervalOptions::default().immediate(false),
    )));

    let onclick_confirm = move |_| {
        let interval = interval.get_value();
        if is_clicked() {
            game_state
                .read_value()
                .send_game_control(game_control.get_value(), user_id);
            is_clicked.update(|v| *v = false);
            (interval.reset)();
            (interval.pause)();
        } else {
            is_clicked.update(|v| *v = true);
            (interval.resume)();
        }
    };

    Effect::new_isomorphic(move |_| {
        let interval = interval.get_value();
        if (interval.counter)() >= 1 {
            is_clicked.update(|v| *v = false);
            (interval.reset)();
            (interval.pause)();
        }
    });

    let pending_slice = create_read_slice(game_state.with_value(|gs| gs.signal), |gs| {
        gs.game_control_pending.clone()
    });

    let cancel = move |_| is_clicked.update(|v| *v = false);
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

    let turn = create_read_slice(game_state.with_value(|gs| gs.signal), |gs| {
        gs.state.turn as i32
    });

    let disabled = move || {
        let game_control = game_control.get_value();
        if game_control.allowed_on_turn(turn()) {
            !matches!(game_control, GameControl::Resign(_))
                && (pending(GameControl::DrawOffer(color))
                    || pending(GameControl::TakebackRequest(color)))
        } else {
            true
        }
    };

    let conditional_button_style = move || {
        if is_clicked() {
            "bg-grasshopper-green hover:bg-green-500"
        } else if pending(game_control.get_value()) {
            "bg-pullbug-teal"
        } else if disabled() {
            ""
        } else {
            "hover:bg-grasshopper-green"
        }
    };

    let conditional_icon_style = move || {
        if disabled() {
            "fill-slate-500"
        } else {
            ""
        }
    };

    view! {
        <div class=move || format!("relative {}", hidden())>
            <button
                title=title
                on:click=onclick_confirm
                prop:disabled=disabled

                class=move || {
                    format!(
                        "aspect-square rounded-sm relative transform transition-transform duration-300 active:scale-95 {}",
                        conditional_button_style(),
                    )
                }
            >

                <Icon
                    icon=icon
                    attr:class=move || {
                        format!("h-6 w-6 lg:h-8 lg:w-8 {}", conditional_icon_style())
                    }
                />

            </button>
            <Show when=is_clicked>
                <button
                    title="Cancel"
                    on:click=cancel
                    class="absolute ml-1 bg-red-700 rounded-sm duration-300 aspect-square hover:bg-ladybug-red"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="w-6 h-6 lg:h-8 lg:w-8" />
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
