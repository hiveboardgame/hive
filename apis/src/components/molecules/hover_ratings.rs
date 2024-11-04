use crate::components::atoms::rating::RatingWithIcon;
use crate::responses::UserResponse;
use leptos::*;
use shared_types::GameSpeed;

#[component]
pub fn HoverRating(user: StoredValue<UserResponse>) -> impl IntoView {
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user().ratings.get(speed) {
                view! { <RatingWithIcon rating=store_value(rating.clone()) /> }
            } else {
                "".into_view()
            }
        })
        .collect_view();
    view! {
        <div class="absolute bottom-0 -left-24 z-40 p-2 rounded bg-even-light dark:bg-gray-950">
            {ratings}
        </div>
    }
}
