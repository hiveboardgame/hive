use leptos::*;
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
            ws.send_with_str("Hi from new WS");
        } else {
            log!("empty WS");
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[inline]
        fn use_websocket_inner() -> HiveWebSocket {
            use_context::<HiveWebSocket>().expect("there to be a `HiveWebSocket` provided")
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

        #[inline]
        fn provide_websocket_inner(url: &str) -> Result<(), JsValue> {
            log!("wasm32");
            if use_context::<HiveWebSocket>().is_none() {
                log!("creating context");
                let ws = WebSocket::new(url)?;
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
