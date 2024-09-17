use super::response_handler::handle_response;
use crate::common::{ClientRequest, CommonMessage};
use crate::functions::hostname::hostname_and_port;
use codee::binary::MsgpackSerdeCodec;
use lazy_static::lazy_static;
use leptos::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::*;
use regex::Regex;
use std::rc::Rc;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<CommonMessage>>,
    send: Rc<dyn Fn(&CommonMessage)>,
    pub ready_state: Signal<ConnectionReadyState>,
    open: Rc<dyn Fn()>,
    close: Rc<dyn Fn()>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<CommonMessage>>,
        send: Rc<dyn Fn(&CommonMessage)>,
        ready_state: Signal<ConnectionReadyState>,
        open: Rc<dyn Fn()>,
        close: Rc<dyn Fn()>,
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
        let message = CommonMessage::Client(message.clone());
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

fn on_message_callback(m: &CommonMessage) {
    handle_response(m);
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
    } = use_websocket_with_options::<CommonMessage, CommonMessage, MsgpackSerdeCodec>(
        &url,
        UseWebSocketOptions::default()
            .on_message(on_message_callback)
            .immediate(false),
    );
    provide_context(WebsocketContext::new(
        message,
        Rc::new(send),
        ready_state,
        Rc::new(open),
        Rc::new(close),
    ));
}
