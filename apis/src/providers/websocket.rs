use crate::common::{ClientRequest, ServerResult};
use crate::functions::hostname::hostname_and_port;
use crate::websocket::client_handlers::response_handler::handle_response;
use codee::binary::MsgpackSerdeCodec;
use leptos::prelude::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<ServerResult>>,
    send: Arc<dyn Fn(&ClientRequest) + Send + Sync>,
    pub ready_state: Signal<ConnectionReadyState>,
    open: Arc<dyn Fn() + Send + Sync>,
    close: Arc<dyn Fn() + Send + Sync>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<ServerResult>>,
        send: Arc<dyn Fn(&ClientRequest) + Send + Sync>,
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
        (self.send)(message)
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

fn on_message_callback(m: &ServerResult) {
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
    // log!("Establishing new websocket connection");
    let owner = Owner::current().unwrap();
    let UseWebSocketReturn {
        message,
        send,
        ready_state,
        open,
        close,
        ..
    } = use_websocket_with_options::<ClientRequest, ServerResult, MsgpackSerdeCodec, _, _>(
        &url,
        UseWebSocketOptions::default()
            .on_message(move |ms| owner.with(|| on_message_callback(ms)))
            .immediate(true),
    );
    provide_context(WebsocketContext::new(
        message,
        Arc::new(send),
        ready_state,
        Arc::new(open),
        Arc::new(close),
    ));
}
