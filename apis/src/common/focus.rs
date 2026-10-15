#[cfg(not(feature = "ssr"))]
use leptos::leptos_dom::helpers::{document, request_animation_frame, window};
use leptos::prelude::*;
#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;
#[cfg(not(feature = "ssr"))]
use web_sys::HtmlElement;

#[cfg(not(feature = "ssr"))]
pub(crate) fn focus_after_render(selector: String, mounted: ArcRwSignal<bool>) {
    let Ok(expected_path) = window().location().pathname() else {
        return;
    };
    request_animation_frame(move || {
        request_animation_frame(move || {
            if !mounted.get_untracked()
                || window().location().pathname().ok().as_deref() != Some(expected_path.as_str())
            {
                return;
            }
            let Ok(Some(element)) = document().query_selector(&selector) else {
                return;
            };
            if let Ok(element) = element.dyn_into::<HtmlElement>() {
                let _ = element.focus();
            }
        });
    });
}

#[cfg(feature = "ssr")]
pub(crate) fn focus_after_render(_selector: String, _mounted: ArcRwSignal<bool>) {}
