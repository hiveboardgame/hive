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
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingContext>();
    let auth_context = expect_context::<AuthContext>();
    let online_users = expect_context::<OnlineUsersSignal>();
    let username = Signal::derive(move || username.clone());
    let user_is_player = move || match auth_context.user.get() {
        Some(Ok(user)) => user.username == username(),
        _ => false,
    };
    let user_has_ws = move || {
        Utc::now()
            .signed_duration_since(ping.last_updated.get_untracked())
            .num_seconds()
            < 5
            && matches!(websocket.ready_state.get(), ConnectionReadyState::Open)
    };

    let icon_style = move || {
        if user_is_player() {
            if user_has_ws() {
                "fill-grasshopper-green"
            } else {
                "w-6 h-6 fill-ladybug-red"
            }
        } else {
            match (online_users.signal)().username_status.get(&username()) {
                Some(UserStatus::Online) => "fill-grasshopper-green",

                // TODO: Figure out away Some(UserStatus::Away) => ....
                _ => "fill-slate-400",
            }
        }
    };
    view! {
        <Icon
            icon=icondata::BiCircleSolid
            prop:class=move || format!("mr-1 pb-[2px] {}", icon_style())
        />
    }
}
