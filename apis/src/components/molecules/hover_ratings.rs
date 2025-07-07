use crate::components::atoms::rating::RatingWithIcon;
use crate::responses::UserResponse;
use leptos::either::Either;
use leptos::html;
use leptos::prelude::*;
use shared_types::GameSpeed;

#[component]
pub fn HoverRating(user: UserResponse, anchor_ref: NodeRef<html::A>) -> impl IntoView {
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

    let position_vars = move || {
        if let Some(element) = anchor_ref.get() {
            let rect = element.get_bounding_client_rect();
            let x = rect.left() - 64.0; // 64px to the left (-left-16)
            let y = rect.bottom() + 2.0; // Below the element with small gap
            format!("--popup-x: {x}px; --popup-y: {y}px;")
        } else {
            "--popup-x: 0px; --popup-y: 0px;".to_string()
        }
    };

    view! {
        <div
            class="fixed z-50 p-2 rounded bg-even-light dark:bg-gray-950 pointer-events-none left-[var(--popup-x)] top-[var(--popup-y)]"
            style=position_vars
        >
            {ratings}
        </div>
    }
}
