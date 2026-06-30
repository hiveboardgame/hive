use crate::{
    common::UserAction,
    components::molecules::user_search::UserSearch,
    providers::online_users::OnlineUsersSignal,
};
use leptos::prelude::*;

#[component]
pub fn OnlineUsers() -> impl IntoView {
    let online_users = expect_context::<OnlineUsersSignal>();
    let fallback_users =
        Signal::derive(move || online_users.signal.with(|ou| ou.username_user.clone()));

    view! {
        <div class="my-2 w-full min-w-0 shrink-0">
            <UserSearch fallback_users=fallback_users actions=vec![UserAction::Challenge] />
        </div>
    }
}
