use crate::{
    common::server_result::UserStatus,
    providers::{
        auth_context::AuthContext, users::UserSignal, ping::PingSignal,
        websocket::context::WebsocketContext,
    },
};
use chrono::Utc;
use leptos::*;
use leptos_icons::*;
use leptos_use::core::ConnectionReadyState;

#[component]
pub fn StatusIndicator(username: String) -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingSignal>();
    let auth_context = expect_context::<AuthContext>();
    let online_users = expect_context::<UserSignal>();
    let username = store_value(username);
    let user_is_player = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => user.username == username(),
        _ => false,
    };
    let user_has_ws = move || {
        Utc::now()
            .signed_duration_since(ping.signal.get().last_update)
            .num_seconds()
            < 5
            && matches!(websocket.ready_state.get(), ConnectionReadyState::Open)
    };

    let icon_color = move || {
        if user_is_player() {
            if user_has_ws() {
                "fill-grasshopper-green"
            } else {
                "fill-ladybug-red"
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
            class=TextProp::from(move || format!("mr-1 pb-[2px] {}", icon_color()))
        />
    }
}
