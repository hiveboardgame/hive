use crate::{
    common::{
        game_action::GameAction,
        server_result::{
            GameActionResponse,
            ServerMessage::{self},
            ServerResult,
        },
    },
    functions::hostname::hostname_and_port,
    providers::{auth_context::AuthContext, game_state::GameStateSignal},
};
use hive_lib::{
    game_control::GameControl, game_result::GameResult, game_status::GameStatus, history::History,
    state::State, turn::Turn,
};
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
        Ok(ServerResult::Ok(ServerMessage::UserStatus(update))) => {
            log!("{update:?}");
        }
        Ok(ServerResult::Ok(ServerMessage::GameUpdate(gar))) => {
            log!("Got a game action response message: {:?}", gar);
            let mut game_state = expect_context::<GameStateSignal>();
            let auth_context = expect_context::<AuthContext>();
            let user_uuid = move || match untrack(auth_context.user) {
                Some(Ok(Some(user))) => Some(user.id),
                _ => None,
            };

            match gar.game_action {
                GameAction::Move(ref turn) => {
                    log!("Playing turn: {turn}");
                    game_state.clear_gc();
                    if Some(gar.user_id) == user_uuid() {
                        log!("Skipping own turn");
                        return;
                    }
                    match turn {
                        Turn::Spawn(piece, position) | Turn::Move(piece, position) => {
                            game_state.play_turn(*piece, *position)
                        }
                        _ => unreachable!(),
                    };
                }
                GameAction::Control(ref game_control) => {
                    log!("Frontend got game_control {}", game_control);
                    game_state.set_pending_gc(game_control.clone());
                    match game_control {
                        GameControl::Abort(_) => {
                            // TODO: Once we have notifications tell the user the game was aborted
                            let navigate = leptos_router::use_navigate();
                            navigate("/", Default::default());
                        }
                        GameControl::DrawAccept(_) => {
                            game_state.set_game_status(GameStatus::Finished(GameResult::Draw))
                        }
                        GameControl::Resign(color) => game_state.set_game_status(
                            GameStatus::Finished(GameResult::Winner(color.opposite_color())),
                        ),
                        GameControl::TakebackAccept(_) => reset_game_state(&gar),
                        _ => {}
                    }
                }
                GameAction::Join => {
                    if Some(gar.user_id) != user_uuid() {
                        log!("{} joined", gar.username);
                        return;
                    }
                    log!("joined the game, reconstructing game state");
                    reset_game_state(&gar);
                    // TODO: @leex try this again once the play page works correctly.
                    if let Some((_turn, gc)) = gar.game.game_control_history.last() {
                        log!("Got a GC: {}", gc);
                        match gc {
                            GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
                                game_state.set_pending_gc(gc.clone())
                            }
                            _ => {}
                        }
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
                reset_game_state(&gar);
            }
        }
        Ok(ServerResult::Err(e)) => log!("Got error from server: {e}"),
        Err(e) => log!("Can't parse: {m}, error is: {e}"),
        foo => {
            log!("Got {foo:?} which is currently still unimplemented");
        } // GameRequiresAction, UserStatusChange, ...
    }
}

fn reset_game_state(gar: &GameActionResponse) {
    let mut game_state = expect_context::<GameStateSignal>();
    let mut history = History::new();
    history.moves = gar.game.history.to_owned();
    history.game_type = gar.game.game_type.to_owned();
    if let GameStatus::Finished(result) = &gar.game.game_status {
        history.result = result.to_owned();
    }
    if let Ok(state) = State::new_from_history(&history) {
        game_state.set_state(state, gar.game.black_player.uid, gar.game.white_player.uid);
    }
    // TODO: check if there an anunsered gc and set it
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
