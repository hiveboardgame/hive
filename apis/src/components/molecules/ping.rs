use crate::providers::ping::PingSignal;
use crate::providers::web_socket::WebsocketContext;
use leptos::*;
use leptos_icons::{
    BiIcon::{BiNoSignalRegular, BiSignal5Regular},
    Icon,
};
use leptos_use::core::ConnectionReadyState;

#[component]
pub fn Ping() -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingSignal>();

    let signal = move || match websocket.ready_state.get() {
        ConnectionReadyState::Open => view! {
            <div class="flex">
                <Icon class="fill-green-400" icon=Icon::from(BiSignal5Regular)/>
                {move || { format!("{}ms", ping.signal.get().ping_duration.num_milliseconds()) }}
            </div>
        }
        .into_view(),
        _ => view! { <Icon class="fill-red-400" icon=Icon::from(BiNoSignalRegular)/> }.into_view(),
    };

    signal.into_view()
}
