use crate::components::atoms::rating::RatingWithIcon;
use crate::responses::UserResponse;
use leptos::either::Either;
use leptos::prelude::*;
use shared_types::GameSpeed;

#[component]
pub fn HoverRating(user: UserResponse) -> impl IntoView {
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user.ratings.get(speed) {
                Either::Left(view! { <RatingWithIcon rating=StoredValue::new(rating.clone()) /> })
            } else {
                Either::Right("")
            }
        })
        .collect_view();
    view! {
        <div class="absolute bottom-0 -left-24 z-40 p-2 rounded bg-even-light dark:bg-gray-950">
            {ratings}
        </div>
    }
}
