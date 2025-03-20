use leptos::prelude::*;
use shared_types::PrettyString;

#[component]
pub fn TypedSelectOption<T: PrettyString + Clone + ToString +'static + std::cmp::PartialEq> (
    variant: T,
    value: RwSignal<T>,
) -> impl IntoView {
    view! {
        <option value=variant selected=move || value.get() == variant>
            {variant.pretty_string()}
        </option>
    }
}
