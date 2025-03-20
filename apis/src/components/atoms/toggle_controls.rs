use crate::components::layouts::base_layout::ControlsSignal;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ToggleControls() -> impl IntoView {
    let controls_signal = expect_context::<ControlsSignal>();
    let toggle_controls = move |_| controls_signal.hidden.update(|b| *b = !*b);
    let icon = move || {
        if controls_signal.hidden.get() {
            view! { <Icon icon=icondata::BiDownArrowSolid attr:class="w-4 h-4" /> }
        } else {
            view! { <Icon icon=icondata::BiUpArrowSolid attr:class="w-4 h-4" /> }
        }
    };
    let title = move || {
        if controls_signal.hidden.get() {
            "Show controls"
        } else {
            "Hide Controls"
        }
    };

    let button_color = move || {
        if controls_signal.notify.get() && controls_signal.hidden.get() {
            "bg-ladybug-red"
        } else {
            "bg-button-dawn dark:bg-button-twilight"
        }
    };

    view! {
        <button
            title=title
            on:click=toggle_controls
            class=move || {
                format!(
                    "{} px-4 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform hover:bg-pillbug-teal active:scale-95",
                    button_color(),
                )
            }
        >

            {icon}
        </button>
    }
}
