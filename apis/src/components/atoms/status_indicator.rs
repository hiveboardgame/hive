use crate::{
    common::UserStatus,
    providers::{
        ClientApi, AuthContext, PingContext, online_users::OnlineUsersSignal,
    }
};
use chrono::Utc;
use leptos::prelude::*;

#[component]
pub fn StatusIndicator(username: String) -> impl IntoView {
    let cloned = username.clone();
    let api = expect_context::<ClientApi>();
    let ws_ready = api.signal_ws_ready();
    let ping = expect_context::<PingContext>();
    let auth_context = expect_context::<AuthContext>();
    let online_users = expect_context::<OnlineUsersSignal>();
    let user_is_player = move || {
        auth_context
            .user
            .with(|u| u.as_ref().is_some_and(|user| user.username == cloned))
    };
    let user_has_ws = move || {
        Utc::now()
            .signed_duration_since(ping.last_updated.get_untracked())
            .num_seconds()
            < 5
            && ws_ready()
    };

    let icon_style = move || {
        let base_classes = "mx-1 pb-[2px]";

        let extra_classes = match (user_is_player(), user_has_ws()) {
            (true, true) => " fill-grasshopper-green",
            (true, false) => " size-6 fill-ladybug-red",
            _ => match online_users
                .signal
                .with(|o| o.username_status.get(&username).cloned())
            {
                Some(UserStatus::Online) => " fill-grasshopper-green",
                // TODO: Handle `Some(UserStatus::Away)`
                _ => " fill-slate-400",
            },
        };

        format!("{base_classes}{extra_classes}")
    };

    view! {
        <svg
            width="1em"
            height="1em"
            viewBox="0 0 24 24"
            fill="currentColor"
            role="graphics-symbol"
            class=icon_style
        >
            <path d="M12 2C6.486 2 2 6.486 2 12s4.486 10 10 10 10-4.486 10-10S17.514 2 12 2z"></path>
        </svg>
    }
}
