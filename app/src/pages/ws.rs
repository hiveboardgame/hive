use crate::common::web_socket::{use_websocket};
use crate::organisms::header::Header;

use leptos::*;


#[component]
pub fn WsPage() -> impl IntoView {
    let ws = use_websocket();
    let send_message = move |_| ws.chat();
    view! {
        <div class="h-screen w-screen overflow-hidden">
            <Header/>
            <div>
                <button on:click=send_message>"Send"</button>
            </div>
        </div>
    }
}
