use crate::providers::game_state::GameStateSignal;
use hive_lib::game_control::GameControl;
use icondata::Icon;
use leptos::*;
use leptos_icons::*;
use leptos_use::{use_interval_with_options, UseIntervalOptions};
use std::rc::Rc;
use uuid::Uuid;

#[component]
pub fn AcceptDenyGc(
    game_control: StoredValue<GameControl>,
    user_id: Uuid,
    #[prop(optional, into)] hidden: MaybeSignal<String>,
) -> impl IntoView {
    let (icon, title) = get_icon_and_title(game_control());

    let button_style = move || match game_control() {
        GameControl::DrawReject(_) | GameControl::TakebackReject(_) => {
            "bg-red-700 hover:bg-ladybug-red absolute"
        }
        _ => "mr-1 bg-grasshopper-green hover:bg-green-500 relative",
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
                format!(
                    "aspect-square rounded-sm transform transition-transform duration-300 active:scale-95 {} {}",
                    button_style(),
                    hidden(),
                )
            }
        >

            <Icon icon=icon class="h-6 w-6 lg:h-8 lg:w-8"/>
        </button>
    }
}

#[component]
pub fn ConfirmButton(
    game_control: StoredValue<GameControl>,
    user_id: Uuid,
    #[prop(optional, into)] hidden: MaybeSignal<String>,
) -> impl IntoView {
    let game_state = store_value(expect_context::<GameStateSignal>());
    let (icon, title) = get_icon_and_title(game_control());
    let color = game_control().color();
    let is_clicked = RwSignal::new(false);
    let interval = store_value(Rc::new(use_interval_with_options(
        5000,
        UseIntervalOptions::default().immediate(false),
    )));

    let onclick_confirm = move |_| {
        if is_clicked() {
            game_state().send_game_control(game_control(), user_id);
            is_clicked.update(|v| *v = false);
            (interval().reset)();
            (interval().pause)();
        } else {
            is_clicked.update(|v| *v = true);
            (interval().resume)();
        }
    };

    create_isomorphic_effect(move |_| {
        if (interval().counter)() >= 1 {
            is_clicked.update(|v| *v = false);
            (interval().reset)();
            (interval().pause)();
        }
    });
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
        if game_control().allowed_on_turn(turn) {
            !matches!(game_control(), GameControl::Resign(_))
                && (pending(GameControl::DrawOffer(color))
                    || pending(GameControl::TakebackRequest(color)))
        } else {
            true
        }
    };

    let conditional_button_style = move || {
        if is_clicked() {
            "bg-grasshopper-green hover:bg-green-500"
        } else if pending(game_control()) {
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
                    class=TextProp::from(move || {
                        format!("h-6 w-6 lg:h-8 lg:w-8 {}", conditional_icon_style())
                    })
                />

            </button>
            <Show when=is_clicked>
                <button
                    title="Cancel"
                    on:click=cancel
                    class="ml-1 aspect-square bg-red-700 hover:bg-ladybug-red rounded-sm absolute duration-300"
                >
                    <Icon icon=icondata::IoCloseSharp class="h-6 w-6 lg:h-8 lg:w-8"/>
                </button>
            </Show>
        </div>
    }
}

fn get_icon_and_title(game_control: GameControl) -> (Icon, &'static str) {
    match game_control {
        GameControl::Abort(_) => (icondata::AiStopOutlined, "Abort"),
        GameControl::DrawAccept(_) => (icondata::FaHandshakeSimpleSolid, "Accept Draw"),
        GameControl::DrawOffer(_) => (icondata::FaHandshakeSimpleSolid, "Offer Draw"),
        GameControl::DrawReject(_) => (icondata::FaHandshakeSimpleSolid, "Reject Draw"),
        GameControl::Resign(_) => (icondata::AiFlagOutlined, "Resign"),
        GameControl::TakebackAccept(_) => (icondata::BiUndoRegular, "Accept Takeback"),
        GameControl::TakebackReject(_) => (icondata::BiUndoRegular, "Reject Takeback"),
        GameControl::TakebackRequest(_) => (icondata::BiUndoRegular, "Request Takeback"),
    }
}

//154:                    class=MaybeProp::derive(TextProp::from(move || {format!("h-6 w-6 lg:h-8 lg:w-8 {}", conditional_icon_style())).into()})
