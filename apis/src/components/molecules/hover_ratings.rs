use crate::components::atoms::rating::RatingWithIcon;
use crate::responses::UserResponse;
use leptos::prelude::*;
use shared_types::GameSpeed;

#[component]
pub fn HoverRating(user: UserResponse) -> impl IntoView {
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user.ratings.get(speed) {
                view! { <RatingWithIcon rating=store_value(rating.clone()) /> }.into_any()
            } else {
                "".into_any()
            }
        })
        .collect_view();
    view! {
        <div class="absolute bottom-0 -left-24 z-40 p-2 rounded bg-even-light dark:bg-gray-950">
            {ratings}
        </div>
    }
}
