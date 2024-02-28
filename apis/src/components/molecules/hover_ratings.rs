use crate::responses::user::UserResponse;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn HoverRating(user: StoredValue<UserResponse>) -> impl IntoView {
    let users_ratings = move || user().ratings.into_iter();
    view! {
        <div class="absolute z-40 bg-even-light dark:bg-even-dark rounded p-2 top-6 left-6">
            <For each=users_ratings key=|rating| (rating.0.to_string()) let:rating>
                <p class="whitespace-nowrap">{rating.0.to_string()} " " {rating.1.rating}</p>
            </For>
        </div>
    }
}
