use crate::common::{render_text_prop, with_class};
use leptos::prelude::*;

#[component]
pub fn EmptyState(
    #[prop(into)] title: TextProp,
    #[prop(optional, into)] message: Option<TextProp>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let has_message = message.is_some();
    let message = message.unwrap_or_default();

    view! {
        <div class=with_class("ui-empty-state", class.unwrap_or_default())>
            <div class="text-sm font-bold text-gray-800 dark:text-gray-100">
                {render_text_prop(title)}
            </div>
            <Show when=move || has_message>
                <div class="mt-1 text-xs">{render_text_prop(message.clone())}</div>
            </Show>
        </div>
    }
}
