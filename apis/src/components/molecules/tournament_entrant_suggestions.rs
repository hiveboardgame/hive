use leptos::prelude::*;

#[component]
pub fn TournamentEntrantSuggestions(
    id: String,
    entrant_names: Signal<Vec<String>>,
) -> impl IntoView {
    view! {
        <datalist id=id>
            {move || {
                let mut names = entrant_names.get();
                names.sort();
                names.dedup();
                names.into_iter().map(|name| view! { <option value=name /> }).collect_view()
            }}
        </datalist>
    }
}
