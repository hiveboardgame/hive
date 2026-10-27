use crate::{common::MoveConfirm, i18n::*, providers::Config};
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn ConfirmModeToggle(game_speed: GameSpeed) -> impl IntoView {
    let i18n = use_i18n();
    let game_speed = Signal::derive(move || game_speed);
    view! {
        <div class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config.move_confirm)}</p>
            <div class="ui-choice-group">
                <ConfirmModeButton move_confirm=MoveConfirm::Single game_speed=game_speed() />
                <ConfirmModeButton move_confirm=MoveConfirm::Double game_speed=game_speed() />
                <ConfirmModeButton move_confirm=MoveConfirm::Clock game_speed=game_speed() />
            </div>
        </div>
    }
}

#[component]
pub fn ConfirmModeButton(move_confirm: MoveConfirm, game_speed: GameSpeed) -> impl IntoView {
    let i18n = use_i18n();
    let move_confirm = Signal::derive(move || move_confirm.clone());
    let game_speed = Signal::derive(move || game_speed);
    let Config(config, set_cookie) = expect_context();
    let icon = match move_confirm() {
        MoveConfirm::Clock => icondata_bi::BiStopwatchRegular,
        MoveConfirm::Double => icondata_tb::TbHandTwoFingersOutline,
        MoveConfirm::Single => icondata_tb::TbHandFingerOutline,
    };
    let is_active = move || {
        config()
            .confirm_mode
            .get(&game_speed())
            .is_some_and(|preferred| *preferred == move_confirm())
    };

    view! {
        <button
            class="ui-choice ui-choice-compact"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            title=move || match move_confirm() {
                MoveConfirm::Clock => t_string!(i18n, user_config.click_on_your_clock).to_string(),
                MoveConfirm::Double => t_string!(i18n, user_config.double_click).to_string(),
                MoveConfirm::Single => t_string!(i18n, user_config.single_click).to_string(),
            }
            on:click=move |_| {
                set_cookie
                    .update(|cookie| {
                        if let Some(cookie) = cookie {
                            cookie.confirm_mode.insert(game_speed(), move_confirm());
                        }
                    });
            }
        >
            <Icon icon=icon attr:class="size-6" />
        </button>
    }
}
