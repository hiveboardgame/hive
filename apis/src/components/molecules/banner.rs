use leptos::{
    html::{HtmlElement, Img},
    prelude::*,
    text_prop::TextProp,
};

#[component]
pub fn Banner<T: IntoView + 'static>(
    title: T,
    #[prop(optional)] text: Option<TextProp>,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] logo: Option<HtmlElement<Img, (), ()>>,
) -> impl IntoView {
    let text_class = format!(
        "text-xl text-center mb-4 {}",
        if text.is_none() { "hidden" } else { "block" }
    );
    view! {
        <div class=format!(
            "flex flex-col items-center justify-center bg-gradient-to-r from-pillbug-teal to-button-dawn text-black p-6 mb-3 xs:p-8 xs:mb-4 rounded-sm {extend_tw_classes}",
        )>
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">{logo} {title}</h1>
            <div class=text_class>{text.map(|t| t.get())}</div>
        </div>
    }
}
