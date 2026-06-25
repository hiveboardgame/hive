use crate::{
    common::UserStatus,
    providers::{
        online_users::OnlineUsersSignal,
        websocket::{ConnectionReadyState, WebsocketContext},
        AuthContext,
        PingContext,
    },
};
use leptos::prelude::*;

#[component]
pub fn StatusIndicator(username: String, deleted: bool) -> impl IntoView {
    if deleted {
        None
    } else {
        let cloned = username.clone();
        let websocket = expect_context::<WebsocketContext>();
        let ping = expect_context::<PingContext>();
        let auth_context = expect_context::<AuthContext>();
        let online_users = expect_context::<OnlineUsersSignal>();
        let user_is_player = move || {
            auth_context
                .user
                .with(|u| u.as_ref().is_some_and(|user| user.username == cloned))
        };
        let user_has_ws = move || {
            ping.is_fresh.get() && matches!(websocket.ready_state.get(), ConnectionReadyState::Open)
        };

        let icon_style = move || {
            let base_classes = "mx-1 size-3 shrink-0";

            let extra_classes = match (user_is_player(), user_has_ws()) {
                (true, true) => " fill-grasshopper-green",
                (true, false) => " fill-ladybug-red",
                _ => match online_users
                    .signal
                    .with(|o| o.username_status.get(&username).cloned())
                {
                    Some(UserStatus::Online) => " fill-grasshopper-green",
                    // TODO: Handle `Some(UserStatus::Away)`
                    _ => " fill-gray-400",
                },
            };

            format!("{base_classes}{extra_classes}")
        };

        Some(view! {
            <svg viewBox="0 0 24 24" fill="currentColor" role="graphics-symbol" class=icon_style>
                <path d="M12 2C6.486 2 2 6.486 2 12s4.486 10 10 10 10-4.486 10-10S17.514 2 12 2z"></path>
            </svg>
        })
    }
}
