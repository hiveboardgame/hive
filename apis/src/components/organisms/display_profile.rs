use crate::{
    components::{atoms::rating::RatingWithIcon, molecules::user_row::UserRow},
    responses::user::UserResponse,
};
use leptos::*;
use shared_types::game_speed::GameSpeed;

#[component]
pub fn DisplayProfile(user: StoredValue<UserResponse>) -> impl IntoView {
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user().ratings.get(speed) {
                view! {
                    <div class="border border-dark dark:border-white p-2">
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
                    <UserRow user=user on_profile=true/>
                </div>
                <div class="flex gap-1 flex-wrap">{ratings}</div>
            </div>

        </div>
    }
}
