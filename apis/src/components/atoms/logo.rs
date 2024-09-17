use leptos::*;

use crate::{components::layouts::base_layout::OrientationSignal, providers::ColorScheme};
#[component]
pub fn Logo(tw_class: &'static str) -> impl IntoView {
    let colorscheme = expect_context::<ColorScheme>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let logo = move || {
        let theme = if colorscheme.prefers_dark.get() {
            "_dark"
        } else {
            ""
        };
        let orientation = if orientation_signal.orientation_vertical.get() {
            "inline"
        } else {
            "stacked"
        };
        format!("/assets/{orientation}_flat{theme}.png")
    };

    view! { <img width="100%" height="100%" src=logo alt="Home" class=tw_class/> }
}
