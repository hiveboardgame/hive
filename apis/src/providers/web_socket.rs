use leptos::logging::log;
use leptos::*;
use leptos_use::core::ConnectionReadyState;
use crate::common::game_action::GameAction;
use leptos_use::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<String>>,
    send: Rc<dyn Fn(&str)>, // use Rc to make it easily cloneable
    pub ready_state: Signal<ConnectionReadyState>,
    open: Rc<dyn Fn()>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<String>>,
        send: Rc<dyn Fn(&str)>,
        ready_state: Signal<ConnectionReadyState>,
        open: Rc<dyn Fn()>,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
            open,
        }
    }

    #[inline(always)]
    pub fn send(&self, message: &str) {
        log!("Sending message: {:?}", message);
        (self.send)(message)
    }

    #[inline(always)]
    pub fn open(&self) {
        log!("Opening connection");
        (self.open)()
    }
}

fn on_message_callback(m: String) {
    match serde_json::from_str::<ServerMessage>(&m) {
        Ok(server_message) => {
            log!("Got a server message: {:?}", server_message);
            match server_message.game_action {
                GameAction::Move(turn) => {
                    log!("Playing turn: {turn}");
                }
                GameAction::Control(_game_control) => {
                    log!("We don't do game controls yet");
                }
                GameAction::Chat(_msg) => {
                    log!("We might do chat at one point");
                }
                GameAction::Join => {}
            }
        }
        Err(e) => log!("Can't parse: {m}, error is: {e}"),
    }
}

use crate::common::server_message::ServerMessage;
use crate::functions::hostname::hostname_and_port;
fn fix_wss(url: &str) -> String {
    let address = hostname_and_port();
    let url = if address.port.is_none() {
        format!("wss://{}{url}", address.hostname)
    } else {
        format!(
            "ws://{}:{}{url}",
            address.hostname,
            address.port.expect("There to be a port")
        )
    };
    url
}

pub fn provide_websocket(url: &str) {
    let url = fix_wss(url);
    log!("Establishing new websocket connection");
    let UseWebsocketReturn {
        message,
        send,
        ready_state,
        open,
        ..
    } = use_websocket_with_options(
        &url,
        UseWebSocketOptions::default().on_message(on_message_callback.clone()),
    );
    provide_context(WebsocketContext::new(
        message,
        Rc::new(send.clone()),
        ready_state,
        Rc::new(open.clone()),
    ));
}
