use crate::common::with_class;
use leptos::prelude::*;

#[component]
pub fn DropdownPanel(
    children: Children,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional, into)] style: Option<Signal<String>>,
) -> impl IntoView {
    let class = with_class("ui-dropdown-panel", class.unwrap_or_default());
    let style = style.map(|style| move || style.get());

    view! {
        <div class=class style=style>
            {children()}
        </div>
    }
}
