use crate::{components::molecules::user_row::UserRow, responses::user::UserResponse};
use leptos::*;

#[component]
pub fn DisplayProfile(user: StoredValue<UserResponse>) -> impl IntoView {
    view! {
        <div class="m-1">
            <div class="flex flex-col items-start ml-3">
                <div class="max-w-fit">
                    <UserRow username=store_value(user().username) rating=user().rating/>
                </div>
                <p>
                    {format!("Wins: {} Draws: {} Losses {}", user().win, user().draw, user().loss)}
                </p>
            </div>

        </div>
    }
}
