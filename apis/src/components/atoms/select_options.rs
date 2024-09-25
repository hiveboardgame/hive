use leptos::*;

#[component]
pub fn SelectOption<T: ToString + Clone + 'static>(
    is: &'static str,
    #[prop(optional)] text: MaybeProp<View>,
    value: RwSignal<T>,
) -> impl IntoView {
    let show = if let Some(text) = text.get() {
        text
    } else {
        is.to_string().into_view()
    };
    view! {
        <option value=is selected=move || value.get().to_string() == is>
            {show}
        </option>
    }
}
