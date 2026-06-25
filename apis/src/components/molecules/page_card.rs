use crate::common::with_class;
use leptos::prelude::*;

#[component]
pub fn PageCard(
    children: Children,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    view! {
        <section class=with_class("ui-page-card", class.unwrap_or_default())>{children()}</section>
    }
}
