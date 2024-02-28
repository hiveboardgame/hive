use crate::{
    components::atoms::{
        direct_challenge_button::DirectChallengeButton, profile_link::ProfileLink, rating::Rating,
        status_indicator::StatusIndicator,
    },
    responses::{rating::RatingResponse, user::UserResponse},
};
use leptos::*;

#[component]
pub fn UserRow(user: StoredValue<UserResponse>) -> impl IntoView {
    // TODO: This needs a hover that displays the users ratings
    view! {
        <div class="flex p-1 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light items-center justify-between h-10">
            <div class="flex w-48 mr-2 justify-between">
                <div class="flex items-center">
                    <StatusIndicator username=user().username/>
                    <ProfileLink username=user().username extend_tw_classes="truncate max-w-[120px]"/>
                </div>
            </div>
            <DirectChallengeButton user=user/>
        </div>
    }
}
