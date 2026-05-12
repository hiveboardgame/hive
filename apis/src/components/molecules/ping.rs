use crate::providers::{
    websocket::{ConnectionReadyState, WebsocketContext},
    PingContext,
};
use leptos::{either::Either, prelude::*};
use leptos_icons::*;

#[component]
pub fn Ping() -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    let ping = expect_context::<PingContext>();

    let signal = move || {
        if !ping.is_fresh.get() {
            Either::Left(
                view! { <Icon attr:class="fill-ladybug-red" icon=icondata_bi::BiNoSignalRegular /> },
            )
        } else {
            match websocket.ready_state.get() {
                ConnectionReadyState::Open => Either::Right(view! {
                    <div class="flex items-center">
                        <Icon
                            attr:class="fill-grasshopper-green"
                            icon=icondata_bi::BiSignal5Regular
                        />
                        {move || { format!("{:.0}ms", ping.ping.get()) }}
                    </div>
                }),
                _ => Either::Left(
                    view! { <Icon attr:class="fill-ladybug-red" icon=icondata_bi::BiNoSignalRegular /> },
                ),
            }
        }
    };

    view! { <div class="m-1 text-black dark:text-white">{signal}</div> }
}
