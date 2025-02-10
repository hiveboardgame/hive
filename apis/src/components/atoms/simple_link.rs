use diesel::sql_types::Text;
use leptos::{html::A, prelude::*, text_prop::TextProp};

#[component]
pub fn SimpleLink(
    link: &'static str,
    #[prop(optional)] text: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <a href=link rel="external" target="_blank" class="text-blue-500 hover:underline">
            {text}
            {children()}
        </a>
    }
}
