use crate::providers::websocket::WebsocketContext;
use crate::providers::PingContext;
use chrono::Utc;
use leptos::*;
use leptos_icons::*;
use leptos_use::core::ConnectionReadyState;

#[component]
pub fn Ping() -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingContext>();

    let signal = move || {
        if Utc::now()
            .signed_duration_since(ping.last_updated.get_untracked())
            .num_seconds()
            >= 5
        {
            view! { <Icon class="fill-ladybug-red" icon=icondata::BiNoSignalRegular/> }.into_view()
        } else {
            match websocket.ready_state.get() {
                ConnectionReadyState::Open => view! {
                    <div class="flex items-center">
                        <Icon class="fill-grasshopper-green" icon=icondata::BiSignal5Regular/>
                        {move || { format!("{:.0}ms", ping.ping.get()) }}
                    </div>
                }
                .into_view(),
                _ => view! { <Icon class="fill-ladybug-red" icon=icondata::BiNoSignalRegular/> }
                    .into_view(),
            }
        }
    };

    view! { <div class="m-1 text-black dark:text-white">{signal}</div> }
}
