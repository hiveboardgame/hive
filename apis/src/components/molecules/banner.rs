use leptos::{
    html::{HtmlElement, Img},
    prelude::*,
    text_prop::TextProp,
};

#[component]
pub fn Banner<T: IntoView + 'static>(
    title: T,
    #[prop(optional)] text: Option<TextProp>,
    #[prop(optional)] logo: Option<HtmlElement<Img, (), ()>>,
) -> impl IntoView {
    let text_class = format!(
        "text-xl text-center mb-4 {}",
        if text.is_none() { "hidden" } else { "block" }
    );
    view! {
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black bg-gradient-to-r rounded-sm from-pillbug-teal to-button-dawn xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">{logo} {title}</h1>
            <div class=text_class>{text.map(|t| t.get())}</div>
        </div>
    }
}
