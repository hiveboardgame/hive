use crate::{common::FlashStyle, hooks::flash_pulse::use_flash_pulse};
use leptos::prelude::*;

#[component]
pub fn WarningFlashOverlay() -> impl IntoView {
    let flashing = use_flash_pulse(FlashStyle::Screen);
    view! {
        <Show when=move || flashing.get()>
            <div class="fixed inset-0 z-50 pointer-events-none warning-flash-overlay"></div>
        </Show>
    }
}
