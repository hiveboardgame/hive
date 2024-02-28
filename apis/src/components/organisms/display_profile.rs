use crate::{components::molecules::user_row::UserRow, responses::user::UserResponse};
use leptos::*;

#[component]
pub fn DisplayProfile(user: StoredValue<UserResponse>) -> impl IntoView {
    let ratings = move || format!("{:?}", user());
    let short = move || {
        format!(
            "Bullet: {} Blitz: {} Rapid: {} Classic: {} Correspondence: {}",
            user().bullet(),
            user().blitz(),
            user().rapid(),
            user().classic(),
            user().correspondence()
        )
    };
    view! {
        <div class="m-1">
            <div class="flex flex-col items-start ml-3">
                <div class="max-w-fit">
                    <UserRow user=user />
                </div>
                <p>
                    {short}
                </p>
                <p>
                    {ratings}
                    //{format!("Wins: {} Draws: {} Losses {}", user().win, user().draw, user().loss)}
                </p>
            </div>

        </div>
    }
}
