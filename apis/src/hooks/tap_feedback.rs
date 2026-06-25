use leptos::{leptos_dom::helpers::set_timeout_with_handle, prelude::*};
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::{Element, PointerEvent};

const TAP_FEEDBACK_DURATION: Duration = Duration::from_millis(180);
const TAP_FEEDBACK_ATTRIBUTE: &str = "data-ui-pressed";

pub(crate) fn use_tap_feedback(selector: &'static str) -> Callback<PointerEvent> {
    Callback::new(move |event: PointerEvent| {
        if event.pointer_type() == "mouse" {
            return;
        }

        let Some(target) = tap_feedback_target(&event, selector) else {
            return;
        };

        let _ = target.set_attribute(TAP_FEEDBACK_ATTRIBUTE, "true");
        let _ = set_timeout_with_handle(
            move || {
                let _ = target.remove_attribute(TAP_FEEDBACK_ATTRIBUTE);
            },
            TAP_FEEDBACK_DURATION,
        );
    })
}

fn tap_feedback_target(event: &PointerEvent, selector: &str) -> Option<Element> {
    event
        .target()
        .and_then(|target| target.dyn_into::<Element>().ok())
        .and_then(|element| element.closest(selector).ok().flatten())
}
