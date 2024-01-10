use leptos::*;

use crate::components::atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator};

#[component]
pub fn UserRow(username: StoredValue<String>, rating: u64) -> impl IntoView {
    view! {
        <li class="flex p-1 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light items-center">
            <StatusIndicator username=username()/>
            <ProfileLink username=username()/>
            <p class="ml-2">{rating}</p>
        </li>
    }
}
