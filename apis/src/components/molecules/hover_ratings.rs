use crate::components::atoms::rating::RatingWithIcon;
use crate::responses::user::UserResponse;
use leptos::*;
use shared_types::game_speed::GameSpeed;

#[component]
pub fn HoverRating(user: StoredValue<UserResponse>) -> impl IntoView {
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user().ratings.get(speed) {
                view! { <RatingWithIcon rating=store_value(rating.clone())/> }
            } else {
                "".into_view()
            }
        })
        .collect_view();
    view! {
        <div class="absolute z-40 bg-even-light dark:bg-even-dark rounded p-2 bottom-0 right-12">
            {ratings}
        </div>
    }
}
