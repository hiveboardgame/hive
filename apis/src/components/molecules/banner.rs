use leptos::{html::Img, *};

#[component]
pub fn Banner(
    title: &'static str,
    #[prop(optional)] text: &'static str,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] logo: Option<HtmlElement<Img>>,
) -> impl IntoView {
    let text_class = format!(
        "text-xl text-center mb-4 {}",
        if text.is_empty() { "hidden" } else { "block" }
    );
    view! {
        <div class=format!("flex flex-col items-center justify-center bg-gradient-to-r from-pillbug-teal to-ant-blue text-dark p-8 mb-4 rounded-sm {extend_tw_classes}")>
            <h1 class="text-4xl font-bold mb-4 flex items-center">{logo} {title}</h1>
            <div class=text_class>{text}</div>
        </div>
    }
}
