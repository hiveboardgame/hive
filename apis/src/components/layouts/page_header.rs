use crate::common::{render_text_prop, with_class};
use leptos::prelude::*;

#[component]
pub fn PageHeader(
    #[prop(into)] title: TextProp,
    #[prop(optional, into)] subtitle: Option<TextProp>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let has_subtitle = subtitle.is_some();
    let subtitle = subtitle.unwrap_or_default();

    view! {
        <header class=with_class("flex flex-col gap-1", class.unwrap_or_default())>
            <h1 class="ui-page-title">{render_text_prop(title)}</h1>
            <Show when=move || has_subtitle>
                <p class="ui-page-subtitle">{render_text_prop(subtitle.clone())}</p>
            </Show>
        </header>
    }
}
