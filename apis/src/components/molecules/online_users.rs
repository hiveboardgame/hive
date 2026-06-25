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
        <div class="my-2 mr-2 ml-2 w-64 lg:mr-0 2xl:mr-2 shrink-0">
            <UserSearch fallback_users=fallback_users actions=vec![UserAction::Challenge] />
        </div>
    }
}
