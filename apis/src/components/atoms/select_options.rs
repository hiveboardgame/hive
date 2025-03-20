use leptos::{either::Either, prelude::*};

#[component]
pub fn SelectOption<T: ToString + Clone + 'static + Send + Sync, S: IntoView + 'static>(
    is: &'static str,
    #[prop(optional)] text: Option<S>,
    value: RwSignal<T>,
) -> impl IntoView {
    let show = if let Some(text) = text {
        Either::Left(text.into_view())
    } else {
        Either::Right(is.into_view())
    };
    view! {
        <option value=is selected=move || value.get().to_string() == is>
            {show}
        </option>
    }
}
