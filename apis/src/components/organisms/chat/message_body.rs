use crate::common::LinkedText;
use leptos::prelude::*;

#[component]
pub fn ChatMessageBody(body: String) -> impl IntoView {
    view! { <LinkedText body /> }
}
