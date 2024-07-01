use leptos::*;

use crate::providers::ColorScheme;

#[component]
pub fn Logo(tw_class: &'static str) -> impl IntoView {
    let colorscheme = expect_context::<ColorScheme>();
    let logo = move || {
        if colorscheme.prefers_dark.get() {
            "/assets/inline_flat_dark.png"
        } else {
            "/assets/inline_flat.png"
        }
    };

    view! {
       <img
           width="100%"
           height="100%"
           src=logo
           alt="Home"
           class=tw_class
       />
    }
}
