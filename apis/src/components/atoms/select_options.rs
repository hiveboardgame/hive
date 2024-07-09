use leptos::*;

#[component]
pub fn SelectOption<T: ToString + Clone + 'static>(
    is: &'static str,
    #[prop(optional)] text: Option<String>,
    value: RwSignal<T>,
) -> impl IntoView {
    let show = if let Some(text) = text { text } else { is.to_string() };
    view! {
        <option value=is selected=move || value.get().to_string() == is>
            {show}
        </option>
    }
}
