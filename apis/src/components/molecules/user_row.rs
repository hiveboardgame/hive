use crate::{
    components::atoms::{
        direct_challenge_button::DirectChallengeButton, profile_link::ProfileLink, rating::Rating,
        status_indicator::StatusIndicator,
    },
    responses::user::UserResponse,
};
use leptos::*;
use shared_types::game_speed::GameSpeed;

#[component]
pub fn UserRow(
    user: StoredValue<UserResponse>,
    #[prop(optional)] game_speed: Option<StoredValue<GameSpeed>>,
) -> impl IntoView {
    let rating = move || {
        if let Some(speed) = game_speed {
            user().ratings.get(&speed()).cloned()
        } else {
            None
        }
    };
    view! {
        <div class="flex p-1 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light items-center justify-between h-10">
            <div class="flex w-48 mr-2 justify-between">

                <div class="flex items-center">
                    <StatusIndicator username=user().username/>
                    <ProfileLink
                        username=user().username
                        extend_tw_classes="truncate max-w-[120px]"
                        user_is_hoverable=user
                    />
                </div>
                <Show when=move || { rating().is_some() }>
                    <Rating rating=rating().expect("Rating is some")/>
                </Show>

            </div>
            <DirectChallengeButton user=user/>
        </div>
    }
}
