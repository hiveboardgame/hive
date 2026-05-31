use crate::{
    common::UserAction,
    components::molecules::user_search::UserSearch,
    i18n::*,
    providers::online_users::OnlineUsersSignal,
};
use leptos::prelude::*;

#[component]
pub fn OnlineUsers() -> impl IntoView {
    let i18n = use_i18n();
    let online_users = expect_context::<OnlineUsersSignal>();
    let fallback_users =
        Signal::derive(move || online_users.signal.with(|ou| ou.username_user.clone()));
    let show_count = Signal::derive(move || {
        online_users
            .signal
            .with(|ou| t_string!(i18n, home.online_players, count = ou.username_user.len()))
    });

    view! {
        <UserSearch
            fallback_users=fallback_users
            show_count
            container_class="flex relative flex-col my-2 mr-2 ml-2 w-64 lg:mr-0 2xl:mr-2 shrink-0"
            actions=vec![UserAction::Challenge]
        />
    }
}
