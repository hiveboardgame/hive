use crate::{
    common::UserStatus,
    providers::{
        online_users::OnlineUsersSignal, websocket::WebsocketContext, AuthContext, PingContext,
    },
};
use chrono::Utc;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::core::ConnectionReadyState;

#[component]
pub fn StatusIndicator(username: String) -> impl IntoView {
    let cloned = username.clone();
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingContext>();
    let auth_context = expect_context::<AuthContext>();
    let online_users = expect_context::<OnlineUsersSignal>();
    let user_is_player = move || {
        auth_context
            .user
            .get()
            .is_some_and(|user| user.username == cloned)
    };
    let user_has_ws = move || {
        Utc::now()
            .signed_duration_since(ping.last_updated.get_untracked())
            .num_seconds()
            < 5
            && matches!(websocket.ready_state.get(), ConnectionReadyState::Open)
    };

    let icon_style = move || {
        let base_classes = "mr-1 pb-[2px]";

        let extra_classes = match (user_is_player(), user_has_ws()) {
            (true, true) => " fill-grasshopper-green",
            (true, false) => " w-6 h-6 fill-ladybug-red",
            _ => match (online_users.signal)().username_status.get(&username) {
                Some(UserStatus::Online) => " fill-grasshopper-green",
                // TODO: Handle `Some(UserStatus::Away)`
                _ => " fill-slate-400",
            },
        };

        format!("{base_classes}{extra_classes}")
    };

    view! { <Icon icon=icondata::BiCircleSolid attr:class=icon_style /> }
}
