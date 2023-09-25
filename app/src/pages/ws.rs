use crate::common::web_socket::{provide_websocket, use_websocket, HiveWebSocket};
use crate::organisms::header::Header;
use crate::organisms::{board::Board, overlay_container::OverlayTabs};
use leptos::*;
use leptos_use::*;

#[component]
pub fn WsPage(cx: Scope) -> impl IntoView {
    let ws = use_websocket(cx);
    let send_message = move |_| ws.chat();
    view! { cx,
        <div class="h-screen w-screen overflow-hidden">
            <Header/>
            <div>
                <button on:click=send_message>"Send"</button>
            </div>
        </div>
    }
}
