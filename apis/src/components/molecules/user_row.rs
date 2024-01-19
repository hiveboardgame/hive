use leptos::*;

use crate::components::atoms::{
    direct_challenge_button::DirectChallenge, profile_link::ProfileLink,
    status_indicator::StatusIndicator,
};

#[component]
pub fn UserRow(username: StoredValue<String>, rating: u64) -> impl IntoView {
    view! {
        <li class="flex p-1 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light items-center justify-between">
            <div class="flex w-48 mr-2 justify-between">
                <div class="flex items-center">
                    <StatusIndicator username=username()/>
                    <ProfileLink username=username() extend_tw_classes="truncate max-w-[120px]"/>
                </div>
                <p class="mx-2">{rating}</p>
            </div>
            <DirectChallenge username=username/>
        </li>
    }
}
