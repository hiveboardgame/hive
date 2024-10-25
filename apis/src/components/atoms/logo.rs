use leptos::*;

use crate::{components::layouts::base_layout::OrientationSignal, providers::Config};
#[component]
pub fn Logo(tw_class: &'static str) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let orientation_signal = expect_context::<OrientationSignal>();
    let logo = move || {
        let theme = if config().prefers_dark { "_dark" } else { "" };
        let orientation = if orientation_signal.orientation_vertical.get() {
            "inline"
        } else {
            "stacked"
        };
        format!("/assets/{orientation}_flat{theme}.png")
    };

    view! { <img width="100%" height="100%" src=logo class=tw_class/> }
}
