use diesel::sql_types::Text;
use leptos::{html::A, prelude::*, text_prop::TextProp};

#[component]
pub fn SimpleLink(
    link: &'static str,
    #[prop(optional)] text: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    //let link = Signal::derive(move || link.to_owned());
    //let link = link.into_any();
   // let text =  text.into_any();
    let children = StoredValue::new(children);
    view! {
        <a href=link rel="external" target="_blank" class="text-blue-500 hover:underline">
            {text}
            {children.get_value()}
        </a>
    }
}
