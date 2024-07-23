use crate::providers::SoundsSignal;
use leptos::*;
use leptos_icons::Icon;
use leptos_router::ActionForm;

#[component]
pub fn SoundToggle() -> impl IntoView {
    let sounds_signal = expect_context::<SoundsSignal>();
    let icon = move || {
        let icon = if sounds_signal.prefers_sound.get() {
            icondata::BiVolumeFullRegular
        } else {
            icondata::BiVolumeMuteRegular
        };
        view! {<Icon icon class="w-4 h-4"/>}
    };
    view! {
        <ActionForm
            action=sounds_signal.action
            class="inline-flex justify-center items-center m-1 rounded">

            <input
                type="hidden"
                name="prefers_sound"
                value=move || (!(sounds_signal.prefers_sound)()).to_string()
            />
            <button
                type="submit"
                class="flex justify-center items-center px-1 py-2 w-full h-full"
            >
                {icon}
            </button>
        </ActionForm>
    }
}
