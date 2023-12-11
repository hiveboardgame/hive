use crate::common::game_action::GameAction;
use crate::common::server_result::{ServerOk::GameUpdate, ServerResult};
use crate::functions::hostname::hostname_and_port;
use crate::providers::auth_context::AuthContext;
use crate::providers::game_state::GameStateSignal;
use hive_lib::game_control::GameControl;
use hive_lib::game_result::GameResult;
use hive_lib::game_status::GameStatus;
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
    close: Rc<dyn Fn()>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<String>>,
        send: Rc<dyn Fn(&str)>,
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
    pub fn send(&self, message: &str) {
        log!("Sending message: {:?}", message);
        (self.send)(message)
    }

    #[inline(always)]
    pub fn open(&self) {
        log!("Opening connection");
        (self.open)()
    }

    #[inline(always)]
    pub fn close(&self) {
        log!("Closing connection");
        (self.close)()
    }
}

fn on_message_callback(m: String) {
    // TODO: @leex this needs to be broken up this is getting out of hand
    match serde_json::from_str::<ServerResult>(&m) {
        Ok(ServerResult::Ok(GameUpdate(gar))) => {
            log!("Got a game action response message: {:?}", gar);
            let mut game_state = expect_context::<GameStateSignal>();
            let auth_context = expect_context::<AuthContext>();
            let user_uuid = move || match untrack(auth_context.user) {
                Some(Ok(Some(user))) => Some(user.id),
                _ => None,
            };

            match gar.game_action {
                GameAction::Move(turn) => {
                    log!("Playing turn: {turn}");
                    game_state.clear_gc();
                    if Some(gar.user_id) == user_uuid() {
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
                GameAction::Control(game_control) => {
                    log!("Setting game_control {}", game_control);
                    game_state.set_pending_gc(game_control.clone());
                    match game_control {
                        GameControl::Abort(_) => {} // we need to work out some kind of redirect
                        GameControl::DrawAccept(_) => {
                            game_state.set_game_status(GameStatus::Finished(GameResult::Draw))
                        }
                        GameControl::Resign(color) => game_state.set_game_status(
                            GameStatus::Finished(GameResult::Winner(color.opposite_color())),
                        ),
                        _ => {}
                    }
                }
                GameAction::Chat(_msg) => {
                    log!("We might do chat at one point");
                }
                GameAction::Join => {
                    if Some(gar.user_id) != user_uuid() {
                        log!("{} joined", gar.username);
                        return;
                    }
                    log!("joined the game, reconstructing game state");
                    let mut history = History::new();
                    history.moves = gar.game.history.clone();
                    history.game_type = gar.game.game_type;
                    if let GameStatus::Finished(ref result) = gar.game.game_status {
                        history.result = result.to_owned();
                    }
                    // TODO: check if there an anunsered gc and set it
                    if let Ok(mut state) = State::new_from_history(&history) {
                        state.tournament = gar.game.tournament_queen_rule;
                        game_state.set_state(
                            state,
                            gar.game.black_player.uid,
                            gar.game.white_player.uid,
                        );
                    }
                    // TODO: @leex
                    // Check here whether it's one of your own GCs and only show it when it's not
                    // your own GC also only if user is a player.
                }
            };
            if game_state.signal.get_untracked().state.history.moves != gar.game.history {
                log!("history diverged, reconstructing please report this as a bug to the developers");
                log!(
                    "game_state history is: {:?}",
                    game_state.signal.get_untracked().state.history.moves
                );
                log!("server_message history is: {:?}", gar.game.history);
                let mut history = History::new();
                history.moves = gar.game.history;
                history.game_type = gar.game.game_type;
                if let GameStatus::Finished(result) = gar.game.game_status {
                    history.result = result;
                }
                if let Ok(state) = State::new_from_history(&history) {
                    game_state.set_state(
                        state,
                        gar.game.black_player.uid,
                        gar.game.white_player.uid,
                    );
                }
            }
        }
        Ok(ServerResult::Err(e)) => log!("Got error from server: {e}"),
        Err(e) => log!("Can't parse: {m}, error is: {e}"),
        _ => unimplemented!(), // GameRequiresAction, UserStatusChange, ...
    }
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
    log!("Establishing new websocket connection");
    let UseWebsocketReturn {
        message,
        send,
        ready_state,
        open,
        close,
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
        Rc::new(close),
    ));
}
