use crate::{
    components::atoms::{
        direct_challenge_button::DirectChallengeButton, profile_link::ProfileLink, rating::Rating,
        status_indicator::StatusIndicator,
    },
    responses::UserResponse,
};
use leptos::*;
use shared_types::GameSpeed;

#[component]
pub fn UserRow(
    user: StoredValue<UserResponse>,
    #[prop(optional)] game_speed: Option<StoredValue<GameSpeed>>,
    #[prop(optional)] on_profile: bool,
) -> impl IntoView {
    let rating = move || {
        if let Some(speed) = game_speed {
            user().ratings.get(&speed()).cloned()
        } else {
            None
        }
    };
    let color = if on_profile {
        "bg-light dark:bg-gray-950"
    } else {
        "dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light"
    };
    let profile_link = move || {
        if on_profile {
            view! {
                <ProfileLink
                    patreon=user().patreon
                    username=user().username
                    extend_tw_classes="truncate max-w-[120px]"
                />
            }
        } else {
            view! {
                <ProfileLink
                    patreon=user().patreon
                    username=user().username
                    extend_tw_classes="truncate max-w-[120px]"
                    user_is_hoverable=user
                />
            }
        }
    };
    view! {
        <div class=format!("flex p-1 items-center justify-between h-10 {color}")>
            <div class="flex justify-between mr-2 w-48">
                <div class="flex items-center">
                    <StatusIndicator username=user().username/>
                    {profile_link()}
                </div>
                <Show when=move || { rating().is_some() }>
                    <Rating rating=rating().expect("Rating is some")/>
                </Show>

            </div>
            <DirectChallengeButton user=user/>
        </div>
    }
}
