use hive_lib::game_control::GameControl;
use leptos::*;
use leptos_icons::{
    AiIcon::AiFlagOutlined, AiIcon::AiStopOutlined, BiIcon::BiUndoRegular, ChIcon::ChCross,
    FaIcon::FaHandshakeSimpleSolid, Icon,
};
use leptos_use::{use_interval_with_options, UseIntervalOptions, UseIntervalReturn};
use uuid::Uuid;

use crate::providers::game_state::GameStateSignal;

#[component]
pub fn AcceptDenyGc(game_control: StoredValue<GameControl>, user_id: Uuid) -> impl IntoView {
    let (icon, title) = get_icon_and_title(game_control());

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
            title=title
            on:click=on_click
            class=move || {
                format!("aspect-square hover:bg-green-500 rounded-sm relative {}", button_style())
            }
        >

            <Icon icon=icon class="h-8 w-8"/>
        </button>
    }
}

#[component]
pub fn ConfirmButton(game_control: StoredValue<GameControl>, user_id: Uuid) -> impl IntoView {
    let game_state = store_value(expect_context::<GameStateSignal>());
    let (icon, title) = get_icon_and_title(game_control());
    let color = game_control().color();
    let is_clicked = RwSignal::new(false);
    let UseIntervalReturn {
        counter,
        reset,
        pause,
        resume,
        ..
    } = use_interval_with_options(5000, UseIntervalOptions::default().immediate(false));
    let resume_clone = resume.clone();
    let pause_clone = pause.clone();
    let reset_clone = reset.clone();

    let onclick_confirm = move |_| {
        if is_clicked() {
            game_state().send_game_control(game_control(), user_id);
            is_clicked.update(|v| *v = false);
            reset_clone();
            pause_clone();
        } else {
            is_clicked.update(|v| *v = true);
            resume_clone();
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
            "bg-green-500 hover:bg-green-400"
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

    create_effect(move |_| {
        if counter() >= 1 {
            is_clicked.update(|v| *v = false);
            reset();
            pause();
        }
    });

    view! {
        <div class="relative">
            <button
                title=title
                on:click=onclick_confirm
                prop:disabled=disabled

                class=move || {
                    format!("aspect-square rounded-sm relative {}", conditional_button_style())
                }
            >

                <Icon
                    icon=icon
                    class=Signal::derive(move || {
                        format!("h-8 w-8 {}", conditional_icon_style())
                    })
                />

            </button>
            <Show when=is_clicked>
                <button
                    title="Cancel"
                    on:click=cancel
                    class="ml-1 aspect-square bg-red-700 hover:bg-red-500 rounded-sm absolute"
                >
                    <Icon icon=Icon::from(ChCross) class="h-8 w-8"/>
                </button>
            </Show>
        </div>
    }
}

fn get_icon_and_title(game_control: GameControl) -> (Icon, &'static str) {
    match game_control {
        GameControl::Abort(_) => (leptos_icons::Icon::Ai(AiStopOutlined), "Abort"),
        GameControl::DrawAccept(_) => (
            leptos_icons::Icon::Fa(FaHandshakeSimpleSolid),
            "Accept Draw",
        ),
        GameControl::DrawOffer(_) => (leptos_icons::Icon::Fa(FaHandshakeSimpleSolid), "Offer Draw"),
        GameControl::DrawReject(_) => (
            leptos_icons::Icon::Fa(FaHandshakeSimpleSolid),
            "Reject Draw",
        ),
        GameControl::Resign(_) => (leptos_icons::Icon::Ai(AiFlagOutlined), "Resign"),
        GameControl::TakebackAccept(_) => {
            (leptos_icons::Icon::Bi(BiUndoRegular), "Accept Takeback")
        }
        GameControl::TakebackReject(_) => {
            (leptos_icons::Icon::Bi(BiUndoRegular), "Reject Takeback")
        }
        GameControl::TakebackRequest(_) => {
            (leptos_icons::Icon::Bi(BiUndoRegular), "Request Takeback")
        }
    }
}
