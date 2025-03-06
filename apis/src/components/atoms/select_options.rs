use leptos::{prelude::*, text_prop::TextProp};

#[component]
pub fn SelectOption<T: ToString + Clone + 'static + Send + Sync>(
    is: &'static str,
    #[prop(optional)] text: Option<TextProp>,
    value: RwSignal<T>,
) -> impl IntoView {
    let show = if let Some(text) = text {
        text.get()
    } else {
        Oco::Borrowed(is)
    };
    view! {
        <option value=is selected=move || value.get().to_string() == is>
            {show}
        </option>
    }
}
