use crate::{
    common::{ClientRequest, ServerResult},
    functions::hostname::{hostname_and_port, Address},
};
use leptos::prelude::*;
use std::sync::Arc;

type SendFn = Arc<dyn Fn(&ClientRequest) + Send + Sync>;
type ControlFn = Arc<dyn Fn() + Send + Sync>;

struct WebsocketParts {
    send: SendFn,
    open: ControlFn,
    close: ControlFn,
    reconnect_now: ControlFn,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum ConnectionReadyState {
    Connecting,
    Open,
    Closing,
    Closed,
}

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<ServerResult>>,
    send: SendFn,
    pub ready_state: Signal<ConnectionReadyState>,
    open: ControlFn,
    close: ControlFn,
    reconnect_now: ControlFn,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<ServerResult>>,
        send: SendFn,
        ready_state: Signal<ConnectionReadyState>,
        open: ControlFn,
        close: ControlFn,
        reconnect_now: ControlFn,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
            open,
            close,
            reconnect_now,
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

    /// Cancel any pending backoff and reconnect immediately if the socket
    /// isn't already `Open`. No-op when the socket is healthy, so it's
    /// safe to call from focus/visibility handlers without churning the
    /// connection.
    #[inline(always)]
    pub fn reconnect_now(&self) {
        (self.reconnect_now)()
    }
}

#[cfg(not(feature = "ssr"))]
mod platform {
    use super::{
        ClientRequest,
        ConnectionReadyState,
        ControlFn,
        SendFn,
        ServerResult,
        WebsocketParts,
    };
    use crate::websocket::client_handlers::response_handler::handle_response;
    use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
    use leptos::{
        ev::{online, pageshow, visibilitychange},
        leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle},
        logging::log,
        prelude::*,
    };
    use leptos_use::{use_document, use_event_listener, use_window};
    use std::{sync::Arc, time::Duration};
    use wasm_bindgen::JsCast;
    use web_sys::{
        js_sys::{ArrayBuffer, Uint8Array},
        BinaryType,
        WebSocket,
    };

    const INITIAL_RECONNECT_DELAY_MS: u64 = 2_000;
    const MAX_RECONNECT_DELAY_MS: u64 = 30_000;
    const CONNECT_TIMEOUT_MS: u64 = 10_000;

    pub(super) fn connect(
        url: String,
        ready_state: RwSignal<ConnectionReadyState>,
        message: RwSignal<Option<ServerResult>>,
    ) -> WebsocketParts {
        let controls = SocketControls::new(url, ready_state, message, Owner::current().unwrap());

        let send: SendFn = Arc::new({
            let controls = controls.clone();
            move |message: &ClientRequest| controls.send(message)
        });

        let open: ControlFn = Arc::new({
            let controls = controls.clone();
            move || controls.open()
        });

        let close: ControlFn = Arc::new({
            let controls = controls.clone();
            move || controls.close()
        });

        let reconnect_now: ControlFn = Arc::new({
            let controls = controls.clone();
            move || controls.reconnect_now()
        });

        install_wake_listeners(&controls);
        controls.open();
        on_cleanup(move || controls.close());

        WebsocketParts {
            send,
            open,
            close,
            reconnect_now,
        }
    }

    fn on_message_callback(m: &ServerResult) {
        handle_response(m.clone());
    }

    fn install_wake_listeners(controls: &SocketControls) {
        let _ = use_event_listener(use_document(), visibilitychange, {
            let controls = controls.clone();
            move |_| {
                if !document().hidden() {
                    controls.reconnect_now();
                }
            }
        });
        let _ = use_event_listener(use_window(), pageshow, {
            let controls = controls.clone();
            move |_| controls.reconnect_now()
        });
        let _ = use_event_listener(use_window(), online, {
            let controls = controls.clone();
            move |_| controls.reconnect_now()
        });
    }

    struct SocketState {
        socket: WebSocket,
        generation: u64,
        _on_open: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
        _on_message: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
        _on_error: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
        _on_close: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::CloseEvent)>,
    }

    impl SocketState {
        fn disconnect(&self) {
            // Detach handlers before close so retired sockets cannot mutate app state.
            self.socket.set_onopen(None);
            self.socket.set_onmessage(None);
            self.socket.set_onerror(None);
            self.socket.set_onclose(None);
            let _ = self.socket.close();
        }
    }

    #[derive(Clone)]
    struct SocketControls {
        url: String,
        socket: StoredValue<Option<SocketState>, LocalStorage>,
        generation: StoredValue<u64>,
        reconnect_attempts: StoredValue<u64>,
        reconnect_timer: StoredValue<Option<TimeoutHandle>>,
        connect_timeout: StoredValue<Option<TimeoutHandle>>,
        manually_closed: StoredValue<bool>,
        ready_state: RwSignal<ConnectionReadyState>,
        message: RwSignal<Option<ServerResult>>,
        owner: Owner,
    }

    impl SocketControls {
        fn new(
            url: String,
            ready_state: RwSignal<ConnectionReadyState>,
            message: RwSignal<Option<ServerResult>>,
            owner: Owner,
        ) -> Self {
            Self {
                url,
                socket: StoredValue::new_local(None),
                generation: StoredValue::new(0),
                reconnect_attempts: StoredValue::new(0),
                reconnect_timer: StoredValue::new(None),
                connect_timeout: StoredValue::new(None),
                manually_closed: StoredValue::new(false),
                ready_state,
                message,
                owner,
            }
        }

        fn open(&self) {
            self.reconnect_attempts.set_value(0);
            self.connect();
        }

        fn connect(&self) {
            self.manually_closed.set_value(false);
            self.clear_reconnect_timer();
            self.clear_connect_timeout();
            self.disconnect_current_socket();
            self.generation.update_value(|generation| *generation += 1);
            let generation = self.generation.get_value();

            let socket = match WebSocket::new(&self.url) {
                Ok(socket) => socket,
                Err(err) => {
                    log!("Could not open websocket: {err:?}");
                    self.ready_state.set(ConnectionReadyState::Closed);
                    self.schedule_reconnect();
                    return;
                }
            };

            socket.set_binary_type(BinaryType::Arraybuffer);
            self.ready_state.set(ConnectionReadyState::Connecting);

            let on_open = {
                let controls = self.clone();
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_| {
                    if controls.is_current(generation) {
                        controls.clear_reconnect_timer();
                        controls.clear_connect_timeout();
                        controls.reconnect_attempts.set_value(0);
                        // First frame: if a bearer token is in memory, send
                        // an Auth request so the backend re-binds the socket
                        // from anonymous to the real user. Done before the
                        // ready_state transition so consumers reacting to
                        // Open never get to send anything ahead of Auth.
                        if let Some(token) = crate::client::get_token() {
                            controls.send(&ClientRequest::Auth(token));
                        }
                        controls.ready_state.set(ConnectionReadyState::Open);
                    }
                })
                    as Box<dyn FnMut(web_sys::Event)>)
            };

            let on_message = {
                let controls = self.clone();
                wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |event: web_sys::MessageEvent| {
                        if !controls.is_current(generation) {
                            return;
                        }

                        let Ok(array_buffer) = event.data().dyn_into::<ArrayBuffer>() else {
                            log!("Ignoring non-binary websocket message");
                            return;
                        };

                        let bytes = Uint8Array::new(&array_buffer).to_vec();
                        let result: Result<ServerResult, _> = MsgpackSerdeCodec::decode(&bytes);
                        match result {
                            Ok(message) => {
                                controls.owner.with(|| {
                                    #[cfg(debug_assertions)]
                                    let zone =
                                        leptos::reactive::diagnostics::SpecialNonReactiveZone::enter();

                                    on_message_callback(&message);

                                    #[cfg(debug_assertions)]
                                    drop(zone);
                                });
                                controls.message.set(Some(message));
                            }
                            Err(err) => {
                                log!("Could not decode websocket message: {err:?}");
                            }
                        }
                    },
                )
                    as Box<dyn FnMut(web_sys::MessageEvent)>)
            };

            let on_error = {
                let controls = self.clone();
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_| {
                    controls.handle_unexpected_disconnect(generation);
                })
                    as Box<dyn FnMut(web_sys::Event)>)
            };

            let on_close = {
                let controls = self.clone();
                wasm_bindgen::closure::Closure::wrap(Box::new(move |_| {
                    controls.handle_unexpected_disconnect(generation);
                })
                    as Box<dyn FnMut(web_sys::CloseEvent)>)
            };

            socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

            self.socket.set_value(Some(SocketState {
                socket,
                generation,
                _on_open: on_open,
                _on_message: on_message,
                _on_error: on_error,
                _on_close: on_close,
            }));
            self.start_connect_timeout(generation);
        }

        fn close(&self) {
            self.manually_closed.set_value(true);
            self.clear_reconnect_timer();
            self.clear_connect_timeout();
            self.reconnect_attempts.set_value(0);
            self.disconnect_current_socket();
            self.ready_state.set(ConnectionReadyState::Closed);
        }

        fn reconnect_now(&self) {
            // Don't override an explicit close (e.g. logout).
            if self.manually_closed.get_value() {
                return;
            }
            // Cancel any pending backoff and reset attempts so we don't
            // come back from a hidden tab still inside a 30s timeout.
            self.clear_reconnect_timer();
            self.reconnect_attempts.set_value(0);
            // Healthy socket: app state and the browser socket agree.
            if self.socket_is_open() {
                return;
            }
            self.connect();
        }

        fn disconnect_current_socket(&self) {
            self.socket.update_value(|socket| {
                if let Some(socket) = socket.take() {
                    socket.disconnect();
                }
            });
        }

        fn send(&self, message: &ClientRequest) {
            let data = match MsgpackSerdeCodec::encode(message) {
                Ok(data) => data,
                Err(err) => {
                    log!("Could not encode websocket message: {err:?}");
                    return;
                }
            };

            self.socket.with_value(|socket| {
                if let Some(socket) = socket.as_ref() {
                    if socket.socket.ready_state() == WebSocket::OPEN {
                        let _ = socket.socket.send_with_u8_array(&data);
                    }
                }
            });
        }

        fn is_current(&self, generation: u64) -> bool {
            self.socket.with_value(|socket| {
                socket
                    .as_ref()
                    .is_some_and(|socket| socket.generation == generation)
            })
        }

        fn socket_is_open(&self) -> bool {
            self.ready_state.get_untracked() == ConnectionReadyState::Open
                && self.socket.with_value(|socket| {
                    socket
                        .as_ref()
                        .is_some_and(|socket| socket.socket.ready_state() == WebSocket::OPEN)
                })
        }

        fn handle_unexpected_disconnect(&self, generation: u64) {
            if self.is_current(generation) {
                self.clear_connect_timeout();
                self.ready_state.set(ConnectionReadyState::Closed);
                self.schedule_reconnect();
            }
        }

        fn start_connect_timeout(&self, generation: u64) {
            let controls = self.clone();
            match set_timeout_with_handle(
                move || {
                    controls.connect_timeout.set_value(None);
                    if controls.manually_closed.get_value() || !controls.is_current(generation) {
                        return;
                    }
                    if controls.ready_state.get_untracked() == ConnectionReadyState::Connecting {
                        log!("Websocket connection timed out; scheduling reconnect");
                        controls.disconnect_current_socket();
                        controls.ready_state.set(ConnectionReadyState::Closed);
                        controls.schedule_reconnect();
                    }
                },
                Duration::from_millis(CONNECT_TIMEOUT_MS),
            ) {
                Ok(timer) => self.connect_timeout.set_value(Some(timer)),
                Err(err) => log!("Could not schedule websocket connect timeout: {err:?}"),
            }
        }

        fn schedule_reconnect(&self) {
            if self.manually_closed.get_value() || self.reconnect_timer.get_value().is_some() {
                return;
            }

            let delay = self.reconnect_delay();
            let controls = self.clone();
            match set_timeout_with_handle(
                move || {
                    controls.reconnect_timer.set_value(None);
                    if controls.manually_closed.get_value() {
                        return;
                    }
                    controls
                        .reconnect_attempts
                        .update_value(|attempts| *attempts = attempts.saturating_add(1));
                    controls.connect();
                },
                Duration::from_millis(delay),
            ) {
                Ok(timer) => self.reconnect_timer.set_value(Some(timer)),
                Err(err) => log!("Could not schedule websocket reconnect: {err:?}"),
            }
        }

        fn reconnect_delay(&self) -> u64 {
            let attempts = self.reconnect_attempts.get_value().min(8);
            let multiplier = 1_u64.checked_shl(attempts as u32).unwrap_or(u64::MAX);
            INITIAL_RECONNECT_DELAY_MS
                .saturating_mul(multiplier)
                .min(MAX_RECONNECT_DELAY_MS)
        }

        fn clear_reconnect_timer(&self) {
            self.reconnect_timer.update_value(|timer| {
                if let Some(timer) = timer.take() {
                    timer.clear();
                }
            });
        }

        fn clear_connect_timeout(&self) {
            self.connect_timeout.update_value(|timer| {
                if let Some(timer) = timer.take() {
                    timer.clear();
                }
            });
        }
    }
}

#[cfg(feature = "ssr")]
mod platform {
    use super::{ClientRequest, ConnectionReadyState, ServerResult, WebsocketParts};
    use leptos::prelude::*;
    use std::sync::Arc;

    pub(super) fn connect(
        url: String,
        _ready_state: RwSignal<ConnectionReadyState>,
        _message: RwSignal<Option<ServerResult>>,
    ) -> WebsocketParts {
        let _ = url;
        WebsocketParts {
            send: Arc::new(|_: &ClientRequest| {}),
            open: Arc::new(|| {}),
            close: Arc::new(|| {}),
            reconnect_now: Arc::new(|| {}),
        }
    }
}

fn fix_wss(url: &str) -> String {
    // Already-absolute (CSR build pointing at a remote backend): pass through.
    if url.starts_with("ws://") || url.starts_with("wss://") {
        return url.to_string();
    }
    let Address { hostname, port } = hostname_and_port();
    match port {
        None => format!("wss://{}{url}", hostname),
        Some(port) => format!("ws://{}:{}{url}", hostname, port),
    }
}

pub fn provide_websocket(url: &str) {
    let url = fix_wss(url);
    let ready_state = RwSignal::new(ConnectionReadyState::Closed);
    let message = RwSignal::new(None::<ServerResult>);
    let WebsocketParts {
        send,
        open,
        close,
        reconnect_now,
    } = platform::connect(url, ready_state, message);

    provide_context(WebsocketContext::new(
        message.into(),
        send,
        ready_state.into(),
        open,
        close,
        reconnect_now,
    ));
}
