use leptos::*;

#[component]
pub fn SelectOption<T: ToString + Clone + 'static>(
    is: &'static str,
    value: RwSignal<T>,
) -> impl IntoView {
    view! {
        <option value=is selected=move || value.get().to_string() == is>
            {is}
        </option>
    }
}
