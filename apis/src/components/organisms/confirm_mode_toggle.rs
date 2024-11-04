use crate::i18n::*;
use crate::{common::MoveConfirm, providers::Config};
use leptos::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn ConfirmModeToggle(game_speed: GameSpeed) -> impl IntoView {
    let i18n = use_i18n();
    let game_speed = store_value(game_speed);
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.move_confirm)}</p>
        <div class="flex">
            <ConfirmModeButton move_confirm=MoveConfirm::Single game_speed=game_speed() />
            <ConfirmModeButton move_confirm=MoveConfirm::Double game_speed=game_speed() />
            <ConfirmModeButton move_confirm=MoveConfirm::Clock game_speed=game_speed() />
        </div>
    }
}

#[component]
pub fn ConfirmModeButton(move_confirm: MoveConfirm, game_speed: GameSpeed) -> impl IntoView {
    let move_confirm = store_value(move_confirm);
    let game_speed = store_value(game_speed);
    let config = expect_context::<Config>().0;
    let (_, set_cookie) = Config::get_cookie();
    let (title, icon) = match move_confirm() {
        MoveConfirm::Clock => ("Click on your clock", icondata::BiStopwatchRegular),
        MoveConfirm::Double => ("Double click", icondata::TbHandTwoFingers),
        MoveConfirm::Single => ("Single click", icondata::TbHandFinger),
    };
    let is_active = move || {
        let inactive_class = "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal";
        config()
            .confirm_mode
            .get(&game_speed())
            .map_or(inactive_class, |preferred| {
                if *preferred == move_confirm() {
                    "bg-pillbug-teal"
                } else {
                    inactive_class
                }
            })
    };

    view! {
        <div class="inline-flex justify-center items-center m-1 text-base font-medium rounded-md border border-transparent shadow cursor-pointer">

            <button
                class=move || {
                    format!(
                        "w-full h-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

                title=title
                on:click=move |_| {
                    set_cookie
                        .update(|cookie| {
                            if let Some(cookie) = cookie {
                                cookie.confirm_mode.insert(game_speed(), move_confirm());
                            }
                        });
                }
            >

                <Icon icon=icon class="w-6 h-6" />
            </button>
        </div>
    }
}
