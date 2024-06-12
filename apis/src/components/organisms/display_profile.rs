use crate::{
    common::UserAction, components::{atoms::rating::RatingWithIcon, molecules::user_row::UserRow}, responses::UserResponse
};
use leptos::*;
use shared_types::GameSpeed;

#[component]
pub fn DisplayProfile(user: StoredValue<UserResponse>) -> impl IntoView {
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user().ratings.get(speed) {
                view! {
                    <div class="p-2 border border-dark dark:border-white">
                        <RatingWithIcon rating=store_value(rating.clone())/>
                        <div>{format!("Total: {}", rating.played)}</div>
                        <div>{format!("Wins: {}", rating.win)}</div>
                        <div>{format!("Losses: {}", rating.loss)}</div>
                        <div>{format!("Draws: {}", rating.draw)}</div>
                    </div>
                }
                .into_view()
            } else {
                "".into_view()
            }
        })
        .collect_view();

    view! {
        <div class="m-1">
            <div class="flex flex-col items-start ml-3">
                <div class="max-w-fit">
                    <UserRow actions=vec![UserAction::Challenge] user=user on_profile=true/>
                </div>
                <div class="flex flex-wrap gap-1">{ratings}</div>
            </div>

        </div>
    }
}
