use crate::providers::{
    websocket::{ConnectionReadyState, WebsocketContext},
    PingContext,
};
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn Ping() -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingContext>();

    let connected =
        move || ping.is_fresh.get() && websocket.ready_state.get() == ConnectionReadyState::Open;
    let status_class = move || {
        if connected() {
            "ui-dropdown-status ui-dropdown-status-ok"
        } else {
            "ui-dropdown-status ui-dropdown-status-danger"
        }
    };
    let status_label = move || {
        if connected() {
            "Connected"
        } else {
            "Disconnected"
        }
    };
    let status_value = move || {
        if connected() {
            format!("{:.0}ms", ping.ping.get())
        } else {
            "offline".to_string()
        }
    };
    let status_icon = move || {
        let icon = if connected() {
            icondata_bi::BiSignal5Regular
        } else {
            icondata_bi::BiNoSignalRegular
        };

        view! { <Icon attr:class="size-4" icon /> }
    };

    view! {
        <div class=status_class>
            {status_icon} <span>{status_label}</span>
            <span class="ml-auto font-mono text-xs tabular-nums">{status_value}</span>
        </div>
    }
}
