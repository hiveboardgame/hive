use hive_lib::game_control::GameControl;
use leptos::*;
use leptos_icons::{
    AiIcon::AiFlagOutlined, AiIcon::AiStopOutlined, BiIcon::BiUndoRegular, ChIcon::ChCross,
    FaIcon::FaHandshakeSimpleSolid, Icon,
};
use uuid::Uuid;

use crate::providers::game_state::GameStateSignal;

#[component]
pub fn AcceptDenyGc(game_control: StoredValue<GameControl>, user_id: Uuid) -> impl IntoView {
    let icon = get_icon(game_control());

    let button_style = move || match game_control() {
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {
            "bg-red-700 hover:bg-red-500 absolute"
        }
        _ => "mr-1 bg-green-700 hover:bg-green-500 relative",
    };

    let on_click = move |_| {
        let game_state = expect_context::<GameStateSignal>();
        game_state.send_game_control(game_control(), user_id);
    };
    view! {
        <button
            on:click=on_click
            class=move || {
                format!("aspect-square hover:bg-green-500 rounded-sm relative {}", button_style())
            }
        >

            <Icon icon=icon class="h-[2vw] w-[2vw]"/>
        </button>
    }
}

#[component]
pub fn ConfirmButton(game_control: StoredValue<GameControl>, user_id: Uuid) -> impl IntoView {
    let game_state = store_value(expect_context::<GameStateSignal>());
    let icon = get_icon(game_control());
    let color = game_control().color();
    let is_clicked = RwSignal::new(false);
    let onclick_confirm = move |_| {
        if is_clicked() {
            game_state().send_game_control(game_control(), user_id);
            is_clicked.update(|v| *v = false);
        } else {
            is_clicked.update(|v| *v = true);
        }
    };
    let cancel = move |_| is_clicked.update(|v| *v = false);
    let pending =
        move |game_control: GameControl| match (game_state().signal)().game_control_pending {
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
        let turn = (game_state().signal)().state.turn as i32;
        if game_control().allowed_on_turn(turn, color) {
            !matches!(game_control(), GameControl::Resign(_))
                && (pending(GameControl::DrawOffer(color))
                    || pending(GameControl::TakebackRequest(color)))
        } else {
            true
        }
    };

    let conditional_button_style = move || {
        if is_clicked() {
            "bg-red-700 hover:bg-red-500"
        } else if pending(game_control()) {
            "bg-cyan-400"
        } else if disabled() {
            ""
        } else {
            "hover:bg-green-500"
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
        <div class="relative">
            <button
                on:click=onclick_confirm
                prop:disabled=disabled

                class=move || {
                    format!("aspect-square rounded-sm relative {}", conditional_button_style())
                }
            >

                <Icon
                    icon=icon
                    class=Signal::derive(move || {
                        format!("h-[2vw] w-[2vw] {}", conditional_icon_style())
                    })
                />

            </button>
            <Show when=is_clicked>
                <button
                    on:click=cancel
                    class="ml-1 aspect-square bg-red-700 hover:bg-green-500 rounded-sm absolute"
                >

                    <Icon icon=Icon::from(ChCross) class="h-[2vw] w-[2vw]"/>
                </button>
            </Show>
        </div>
    }
}

fn get_icon(game_control: GameControl) -> Icon {
    match game_control {
        GameControl::Abort(_) => leptos_icons::Icon::Ai(AiStopOutlined),
        GameControl::DrawAccept(_) | GameControl::DrawOffer(_) | GameControl::DrawReject(_) => {
            leptos_icons::Icon::Fa(FaHandshakeSimpleSolid)
        }
        GameControl::Resign(_) => leptos_icons::Icon::Ai(AiFlagOutlined),
        GameControl::TakebackAccept(_)
        | GameControl::TakebackReject(_)
        | GameControl::TakebackRequest(_) => leptos_icons::Icon::Bi(BiUndoRegular),
    }
}
