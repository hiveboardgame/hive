use crate::common::{ClientRequest, WebsocketMessage};
use crate::functions::hostname::hostname_and_port;
use crate::websocket::client_handlers::response_handler::handle_response;
use codee::binary::MsgpackSerdeCodec;
use lazy_static::lazy_static;
use leptos::prelude::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::*;
use regex::Regex;
use std::sync::Arc;
lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<WebsocketMessage>>,
    send: Arc<dyn Fn(&WebsocketMessage) + Send + Sync>,
    pub ready_state: Signal<ConnectionReadyState>,
    open: Arc<dyn Fn() + Send + Sync>,
    close: Arc<dyn Fn() + Send + Sync>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<WebsocketMessage>>,
        send: Arc<dyn Fn(&WebsocketMessage) + Send + Sync>,
        ready_state: Signal<ConnectionReadyState>,
        open: Arc<dyn Fn() + Send + Sync>,
        close: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
            open,
            close,
        }
    }

    #[inline(always)]
    pub fn send(&self, message: &ClientRequest) {
        let message = WebsocketMessage::Client(message.clone());
        (self.send)(&message)
    }

    #[inline(always)]
    pub fn open(&self) {
        //log!("Opening connection");
        (self.open)()
    }

    #[inline(always)]
    pub fn close(&self) {
        //log!("Closing connection");
        (self.close)()
    }
}

fn on_message_callback(m: &WebsocketMessage) {
    handle_response(m.clone());
}

fn fix_wss(url: &str) -> String {
    let address = hostname_and_port();

    if address.port.is_none() {
        format!("wss://{}{url}", address.hostname)
    } else {
        format!(
            "ws://{}:{}{url}",
            address.hostname,
            address.port.expect("There to be a port")
        )
    }
}

pub fn provide_websocket(url: &str) {
    let url = fix_wss(url);
    //log!("Establishing new websocket connection");
    let UseWebSocketReturn {
        message,
        send,
        ready_state,
        open,
        close,
        ..
    } = use_websocket_with_options::<WebsocketMessage, WebsocketMessage, MsgpackSerdeCodec>(
        &url,
        UseWebSocketOptions::default()
            .on_message(on_message_callback)
            .immediate(false),
    );
    provide_context(WebsocketContext::new(
        message,
        Arc::new(send),
        ready_state,
        Arc::new(open),
        Arc::new(close),
    ));
}
