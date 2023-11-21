use crate::common::game_action::GameAction;
use crate::providers::auth_context::AuthContext;
use crate::providers::game_state::GameStateSignal;
use hive_lib::history::History;
use hive_lib::state::State;
use hive_lib::turn::Turn;
use leptos::logging::log;
use leptos::*;
use leptos_use::core::ConnectionReadyState;
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
            let mut game_state = expect_context::<GameStateSignal>();
            let auth_context = expect_context::<AuthContext>();
            let user_uuid = move || match untrack(auth_context.user) {
                Some(Ok(Some(user))) => Some(user.id),
                _ => None,
            };

            match server_message.game_action {
                GameAction::Move(turn) => {
                    log!("Playing turn: {turn}");
                    if Some(server_message.user_id) == user_uuid() {
                        log!("Skipping own turn");
                        return;
                    }
                    match turn {
                        Turn::Spawn(piece, position) | Turn::Move(piece, position) => {
                            game_state.play_turn(piece, position)
                        }
                        _ => unreachable!(),
                    };
                }
                GameAction::Control(_game_control) => {
                    log!("We don't do game controls yet");
                }
                GameAction::Chat(_msg) => {
                    log!("We might do chat at one point");
                }
                GameAction::Error(error_message) => {
                    log!("Got error: {error_message}");
                }
                GameAction::Join => {
                    if Some(server_message.user_id) != user_uuid() {
                        log!("{} joined", server_message.username);
                        return;
                    }
                    log!("joined the game, reconstructing game state");
                    let mut history = History::new();
                    history.moves = server_message.game.history.clone();
                    history.game_type = server_message.game.game_type.clone();
                    if let Ok(state) = State::new_from_history(&history) {
                        game_state.set_state(
                            state,
                            server_message.game.black_player.uid,
                            server_message.game.white_player.uid,
                        );
                    }
                }
            };
            if game_state.signal.get_untracked().state.history.moves != server_message.game.history
            {
                log!("history diverged, reconstructing please report this as a bug to the developers");
                log!(
                    "game_state history is: {:?}",
                    game_state.signal.get_untracked().state.history.moves
                );
                log!(
                    "server_message history is: {:?}",
                    server_message.game.history
                );
                let mut history = History::new();
                history.moves = server_message.game.history;
                history.game_type = server_message.game.game_type;
                if let Ok(state) = State::new_from_history(&history) {
                    game_state.set_state(
                        state,
                        server_message.game.black_player.uid,
                        server_message.game.white_player.uid,
                    );
                }
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
        UseWebSocketOptions::default().on_message(on_message_callback),
    );
    provide_context(WebsocketContext::new(
        message,
        Rc::new(send),
        ready_state,
        Rc::new(open),
    ));
}
