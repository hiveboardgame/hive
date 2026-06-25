use crate::components::layouts::base_layout::ControlsSignal;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ToggleControls() -> impl IntoView {
    let controls_signal = expect_context::<ControlsSignal>();
    let toggle_controls = move |_| controls_signal.hidden.update(|b| *b = !*b);
    let icon = move || {
        if controls_signal.hidden.get() {
            icondata_bi::BiDownArrowSolid
        } else {
            icondata_bi::BiUpArrowSolid
        }
    };
    let title = move || {
        if controls_signal.hidden.get() {
            "Show controls"
        } else {
            "Hide Controls"
        }
    };

    let has_alert = move || controls_signal.notify.get() && controls_signal.hidden.get();

    view! {
        <button
            type="button"
            title=title
            on:click=toggle_controls
            class=move || {
                if has_alert() {
                    "ui-header-icon-button ui-header-action-alert"
                } else {
                    "ui-header-icon-button"
                }
            }
        >

            <Icon icon=Signal::derive(icon) attr:class="size-4" />
        </button>
    }
}
