use leptos::logging::log;
use wasm_bindgen::JsValue;
use web_sys::WebSocket;

#[allow(unused_variables)]
pub fn provide_websocket(url: &str) -> Result<(), JsValue> {
    provide_websocket_inner(url)
}

pub fn use_websocket() -> HiveWebSocket {
    use_websocket_inner()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiveWebSocket {
    ws: Option<WebSocket>,
}

impl HiveWebSocket {
    pub fn chat(&self) {
        if let Some(ws) = &self.ws {
            let _ = ws.send_with_str("Hi from new WS");
        } else {
            log!("empty WS");
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use leptos::expect_context;
        #[inline]
        fn use_websocket_inner() -> HiveWebSocket {
            expect_context::<HiveWebSocket>()
        }
    } else {
        #[inline]
        fn use_websocket_inner() -> HiveWebSocket {
            HiveWebSocket{ws: None}
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use leptos::{provide_context, use_context};
        use crate::functions::hostname::hostname_and_port;

        #[inline]
        fn provide_websocket_inner(url: &str) -> Result<(), JsValue> {
            log!("wasm32");
            if use_context::<HiveWebSocket>().is_none() {
                log!("creating context");
                let address = hostname_and_port();
                let url = if address.port.is_none() {
                    format!("wss://{}{url}",address.hostname)}
                else {
                    format!("ws://{}:{}{url}",address.hostname, address.port.expect("There to be a port"))};
                let ws = WebSocket::new(&url)?;
                provide_context(HiveWebSocket{ws: Some(ws)});
            }
            Ok(())
        }
    } else {
        #[inline]
        fn provide_websocket_inner(_url: &str) -> Result<(), JsValue> {
            log!("non wasm32");
            Ok(())
        }
    }
}

