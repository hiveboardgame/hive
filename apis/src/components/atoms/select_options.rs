use leptos::prelude::*;

#[component]
pub fn SelectOption<T: ToString + Clone + 'static + Send + Sync>(
    is: &'static str,
    #[prop(optional)] text: MaybeProp<T>,
    value: RwSignal<T>,
) -> impl IntoView {
    let show = if let Some(text) = text.get() {
        text.to_string()
    } else {
        is.to_string()
    };
    view! {
        <option value=is selected=move || value.get().to_string() == is>
            {show}
        </option>
    }
}
