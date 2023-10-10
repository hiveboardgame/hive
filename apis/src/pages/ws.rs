use crate::providers::web_socket::use_websocket;
use leptos::*;

#[component]
pub fn WsPage() -> impl IntoView {
    let ws = use_websocket();
    let send_message = move |_| ws.chat();
    view! {
        <div class="h-screen w-screen overflow-hidden">
            <div>
                <button on:click=send_message>"Send"</button>
            </div>
        </div>
    }
}
