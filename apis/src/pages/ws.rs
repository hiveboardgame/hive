//use crate::providers::game_state::GameStateSignal;
use crate::providers::web_socket::WebsocketContext;
use leptos::*;

#[component]
pub fn WsPage() -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    //let mut game_state = expect_context::<GameStateSignal>();
    let send_message = move |_| websocket.send("This is a WS message from the WS test page");
    //let send_join = move |_| game_state.join();
    //<div class="">
    //   <button on:click=send_join>"Join"</button>
    //</div>
    view! {
        <div class="">
            <button on:click=send_message>"Send"</button>
        </div>
    }
}

