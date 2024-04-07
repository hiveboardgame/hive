use crate::{common::config_options::MoveConfirm, providers::config::Config};
use leptos::*;
use leptos_icons::Icon;
use leptos_router::ActionForm;

#[component]
pub fn ConfirmModeToggle() -> impl IntoView {
    view! {
        <p class="text-dark dark:text-white m-1">Move confirmation:</p>
        <div class="flex">
            <ConfirmModeButton move_confirm=MoveConfirm::Single/>
            <ConfirmModeButton move_confirm=MoveConfirm::Double/>
            <ConfirmModeButton move_confirm=MoveConfirm::Clock/>
        </div>
    }
}

#[component]
pub fn ConfirmModeButton(move_confirm: MoveConfirm) -> impl IntoView {
    let move_confirm = store_value(move_confirm);
    let config = expect_context::<Config>();
    let (title, icon) = match move_confirm() {
        MoveConfirm::Clock => ("Click on your clock", icondata::BiStopwatchRegular),
        MoveConfirm::Double => ("Double click", icondata::TbHandTwoFingers),
        MoveConfirm::Single => ("Single click", icondata::TbHandFinger),
    };
    let is_active = move || {
        if (config.confirm_mode.preferred_confirm)() == move_confirm() {
            "bg-pillbug-teal"
        } else {
            "bg-ant-blue hover:bg-pillbug-teal"
        }
    };

    view! {
        <ActionForm
            action=config.confirm_mode.action
            class="m-1 inline-flex items-center border border-transparent text-base font-medium rounded-md shadow justify-center cursor-pointer"
        >
            <input type="hidden" name="move_confirm" value=move_confirm().to_string()/>
            <button
                class=move || {
                    format!(
                        "w-full h-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }

                type="submit"
                title=title
            >
                <Icon icon=icon class="h-6 w-6"/>
            </button>
        </ActionForm>
    }
}
