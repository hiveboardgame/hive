use crate::{
    common::{
        game_action::GameAction,
        server_result::{ChallengeUpdate, ServerMessage, ServerResult},
    },
    functions::hostname::hostname_and_port,
    providers::{
        auth_context::AuthContext, challenges::ChallengeStateSignal, game_state::GameStateSignal,
        timer::TimerSignal,
    },
    responses::game::GameResponse,
};
use hive_lib::{
    game_control::GameControl, game_result::GameResult, game_status::GameStatus, history::History,
    state::State, turn::Turn,
};
use lazy_static::lazy_static;
use leptos::logging::log;
use leptos::*;
use leptos_router::{use_navigate, RouterContext};
use leptos_use::core::ConnectionReadyState;
use leptos_use::*;
use regex::Regex;
use std::rc::Rc;
lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

fn current_page_game_id() -> Option<String> {
    let router = expect_context::<RouterContext>();
    if let Some(caps) = NANOID.captures(&(router.pathname().get_untracked())) {
        if let Some(m) = caps.name("nanoid") {
            return Some(m.as_str().to_owned());
        }
    }
    None
}

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
        log!("WS sending message: {:?}", message);
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

            match current_page_game_id() {
                None => return,
                Some(current_id) => {
                    if gar.game_id != current_id {
                        return;
                    }
                }
            }

            match gar.game_action {
                GameAction::Move(ref turn) => {
                    let timer = expect_context::<TimerSignal>();
                    timer.update_from(&gar.game);
                    game_state.clear_gc();
                    if Some(gar.user_id) == user_uuid() {
                        let mut games = game_state.signal.get_untracked().next_games;
                        games.retain(|g| *g != gar.game_id);
                        game_state.set_next_games(games);
                        log!("Skipping own turn");
                        return;
                    }
                    log!("Playing turn: {turn}");
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
                            game_state.set_game_status(GameStatus::Finished(GameResult::Draw));
                            let timer = expect_context::<TimerSignal>();
                            timer.update_from(&gar.game);
                        }
                        GameControl::Resign(color) => {
                            game_state.set_game_status(GameStatus::Finished(GameResult::Winner(
                                color.opposite_color(),
                            )));
                            let timer = expect_context::<TimerSignal>();
                            timer.update_from(&gar.game);
                        }
                        GameControl::TakebackAccept(_) => reset_game_state(&gar.game),
                        _ => {}
                    }
                }
                GameAction::Join => {
                    if Some(gar.user_id) != user_uuid() {
                        log!("{} joined", gar.username);
                        return;
                    }
                    log!("joined the game, reconstructing game state");
                    reset_game_state(&gar.game);
                    let timer = expect_context::<TimerSignal>();
                    timer.update_from(&gar.game);
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
                reset_game_state(&gar.game);
                let timer = expect_context::<TimerSignal>();
                timer.update_from(&gar.game);
            }
        }
        Ok(ServerResult::Ok(ServerMessage::GameActionNotification(games))) => {
            let mut game_state = expect_context::<GameStateSignal>();
            log!("New games: {:?}", games);
            game_state.set_next_games(games);
        }
        Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Challenges(
            new_challanges,
        )))) => {
            for chal in new_challanges.clone() {
                log!("Got new challenge: {}", chal.nanoid);
            }
            let mut challenges = expect_context::<ChallengeStateSignal>();
            let auth_context = expect_context::<AuthContext>();
            let user_uuid = move || match untrack(auth_context.user) {
                Some(Ok(Some(user))) => Some(user.id),
                _ => None,
            };
            log!("User is: {:?}", user_uuid());
            challenges.add(new_challanges, user_uuid());
        }
        Ok(ServerResult::Ok(ServerMessage::GameTimeoutCheck(game))) => {
            let mut game_state = expect_context::<GameStateSignal>();
            if game_state.signal.get_untracked().state.game_status != game.game_status {
                reset_game_state(&game);
                let timer = expect_context::<TimerSignal>();
                timer.update_from(&game);
                if let GameStatus::Finished(_) = game.game_status {
                    let mut games = game_state.signal.get_untracked().next_games;
                    games.retain(|g| *g != game.nanoid);
                    game_state.set_next_games(games);
                }
            }
        }
        Ok(ServerResult::Ok(ServerMessage::GameNew(game_response))) => {
            if game_response.time_mode == "Real Time" {
                let auth_context = expect_context::<AuthContext>();
                let user_uuid = move || match untrack(auth_context.user) {
                    Some(Ok(Some(user))) => Some(user.id),
                    _ => None,
                };
                if let Some(id) = user_uuid() {
                    if id == game_response.white_player.uid || id == game_response.black_player.uid
                    {
                        let navigate = use_navigate();
                        navigate(
                            &format!("/game/{}", game_response.nanoid),
                            Default::default(),
                        );
                    }
                }
            }
        }
        Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Removed(nanoid)))) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.remove(nanoid);
        }
        Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Created(challenge))))
        | Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Direct(challenge)))) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            let auth_context = expect_context::<AuthContext>();
            let user_uuid = move || match untrack(auth_context.user) {
                Some(Ok(Some(user))) => Some(user.id),
                _ => None,
            };
            challenges.add(vec![challenge], user_uuid());
        }
        Ok(ServerResult::Err(e)) => log!("Got error from server: {e}"),
        Err(e) => log!("Can't parse: {m}, error is: {e}"),
        todo => {
            log!("Got {todo:?} which is currently still unimplemented");
        } // GameRequiresAction, UserStatusChange, ...
    }
}

fn reset_game_state(game: &GameResponse) {
    let mut game_state = expect_context::<GameStateSignal>();
    let mut history = History::new();
    history.moves = game.history.to_owned();
    history.game_type = game.game_type.to_owned();
    if let GameStatus::Finished(result) = &game.game_status {
        history.result = result.to_owned();
    }
    if let Ok(state) = State::new_from_history(&history) {
        game_state.set_state(state, game.black_player.uid, game.white_player.uid);
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
