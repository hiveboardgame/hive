use crate::{
    common::UserStatus,
    i18n::*,
    providers::{
        online_users::OnlineUsersSignal,
        websocket::{ConnectionReadyState, WebsocketContext},
        AuthContext,
        PingContext,
    },
};
use leptos::prelude::*;

#[component]
pub fn StatusIndicator(
    username: String,
    deleted: bool,
    #[prop(optional, default = true)] show_unknown: bool,
) -> impl IntoView {
    if deleted {
        None
    } else {
        let cloned = username.clone();
        let i18n = use_i18n();
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

        let status = Signal::derive(move || {
            match (user_is_player(), user_has_ws()) {
                (true, true) => Some(true),
                (true, false) => Some(false),
                _ => match online_users
                    .signal
                    .with(|o| o.username_status.get(&username).cloned())
                {
                    Some(UserStatus::Online) => Some(true),
                    // TODO: Handle `Some(UserStatus::Away)` when it has defined UI semantics.
                    _ => None,
                },
            }
        });
        let icon_style = move || {
            let base_classes = "mx-1 size-3 shrink-0";

            let extra_classes = match status.get() {
                Some(true) => " fill-grasshopper-green",
                Some(false) => " fill-ladybug-red",
                None if !show_unknown => " hidden",
                None => " fill-gray-400",
            };

            format!("{base_classes}{extra_classes}")
        };

        Some(view! {
            <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                role="img"
                class=icon_style
                aria-label=move || match status.get() {
                    Some(true) => t_string!(i18n, tournaments.view.arena.online).to_string(),
                    Some(false) => t_string!(i18n, tournaments.view.arena.offline).to_string(),
                    None => String::new(),
                }
            >
                <title>
                    {move || match status.get() {
                        Some(true) => t_string!(i18n, tournaments.view.arena.online).to_string(),
                        Some(false) => t_string!(i18n, tournaments.view.arena.offline).to_string(),
                        None => String::new(),
                    }}
                </title>
                <path d="M12 2C6.486 2 2 6.486 2 12s4.486 10 10 10 10-4.486 10-10S17.514 2 12 2z"></path>
            </svg>
        })
    }
}
