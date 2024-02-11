use crate::{
    common::{
        game_action::GameAction,
        server_result::{ChallengeUpdate, ServerMessage, ServerResult, UserStatus},
    },
    functions::hostname::hostname_and_port,
    providers::{
        alerts::{AlertType, AlertsContext},
        auth_context::AuthContext,
        challenges::ChallengeStateSignal,
        game_state::GameStateSignal,
        games::GamesSignal,
        navigation_controller::NavigationControllerSignal,
        online_users::OnlineUsersSignal,
        ping::PingSignal,
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
use leptos_router::use_navigate;
use leptos_use::core::ConnectionReadyState;
use leptos_use::*;
use regex::Regex;
use shared_types::time_mode::TimeMode;
use std::rc::Rc;
use std::str::FromStr;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<String>>,
    send: Rc<dyn Fn(&str)>,
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
    let mut game_state = expect_context::<GameStateSignal>();
    let mut games = expect_context::<GamesSignal>();
    let mut ping = expect_context::<PingSignal>();
    match serde_json::from_str::<ServerResult>(&m) {
        Ok(ServerResult::Ok(ServerMessage::Pong { ping_sent, .. })) => {
            ping.update_ping(ping_sent);
        }
        Ok(ServerResult::Ok(ServerMessage::UserStatus(user_update))) => {
            let mut online_users = expect_context::<OnlineUsersSignal>();
            log!("{:?}", user_update);
            match user_update.status {
                UserStatus::Online => online_users.add(
                    user_update.user.expect("User is online"),
                    UserStatus::Online,
                ),
                UserStatus::Offline => online_users.remove(user_update.username),
                UserStatus::Away => todo!("We need to do away in the frontend"),
            }
        }
        Ok(ServerResult::Ok(ServerMessage::GameUpdate(gar))) => {
            log!("Got a game action response message: {:?}", gar);
            if gar.game.finished {
                log!("Removing finished game {}", gar.game.nanoid.clone());
                games.own_games_remove(&gar.game.nanoid);
            } else {
                games.own_games_add(gar.game.to_owned());
            }
            let navigation_controller = expect_context::<NavigationControllerSignal>();
            if let Some(nanoid) = navigation_controller.signal.get_untracked().nanoid {
                if nanoid == gar.game.nanoid {
                    match gar.game_action {
                        GameAction::Move(ref turn) => {
                            let timer = expect_context::<TimerSignal>();
                            timer.update_from(&gar.game);
                            game_state.clear_gc();
                            game_state.set_game_response(gar.game.clone());
                            if game_state.signal.get_untracked().state.history.moves
                                != gar.game.history
                            {
                                match turn {
                                    Turn::Move(piece, position) => {
                                        game_state.play_turn(*piece, *position)
                                    }
                                    _ => unreachable!(),
                                };
                            }
                        }
                        GameAction::Control(ref game_control) => {
                            game_state.set_pending_gc(game_control.clone());
                            match game_control {
                                GameControl::Abort(_) => {
                                    let alerts = expect_context::<AlertsContext>();
                                    games.own_games_remove(&gar.game.nanoid);
                                    alerts.last_alert.update(|v| {
                                        *v = Some(AlertType::Warn(format!(
                                            "{} aborted the game",
                                            gar.username
                                        )));
                                    });
                                    // TODO: Once we have notifications tell the user the game was aborted
                                    let navigate = leptos_router::use_navigate();
                                    navigate("/", Default::default());
                                }
                                GameControl::DrawAccept(_) => {
                                    game_state
                                        .set_game_status(GameStatus::Finished(GameResult::Draw));
                                    game_state.set_game_response(gar.game.clone());
                                    let timer = expect_context::<TimerSignal>();
                                    timer.update_from(&gar.game);
                                }
                                GameControl::Resign(color) => {
                                    game_state.set_game_status(GameStatus::Finished(
                                        GameResult::Winner(color.opposite_color()),
                                    ));
                                    game_state.set_game_response(gar.game.clone());
                                    let timer = expect_context::<TimerSignal>();
                                    timer.update_from(&gar.game);
                                }
                                GameControl::TakebackAccept(_) => {
                                    let timer = expect_context::<TimerSignal>();
                                    timer.update_from(&gar.game);
                                    reset_game_state(&gar.game);
                                }
                                _ => {}
                            }
                        }
                        // GameUpdate(GameAction::Join) is always a direct message
                        GameAction::Join => {
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
            }
        }
        Ok(ServerResult::Ok(ServerMessage::GameActionNotification(new_games))) => {
            log!(
                "Setting game: {:?}",
                new_games
                    .iter()
                    .map(|g| g.nanoid.clone())
                    .collect::<Vec<String>>()
            );
            games.own_games_set(new_games);
        }
        Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Challenges(
            new_challanges,
        )))) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.add(new_challanges);
        }
        Ok(ServerResult::Ok(ServerMessage::GameTimeoutCheck(game))) => {
            let mut game_state = expect_context::<GameStateSignal>();
            game_state.set_game_response(game.clone());
            if game_state.signal.get_untracked().state.game_status != game.game_status {
                reset_game_state(&game);
                let timer = expect_context::<TimerSignal>();
                timer.update_from(&game);
            }
        }
        Ok(ServerResult::Ok(ServerMessage::GameTimedOut(nanoid))) => {
            games.own_games_remove(&nanoid);
            games.live_games_remove(&nanoid);
        }
        Ok(ServerResult::Ok(ServerMessage::GameNew(game_response))) => {
            games.own_games_add(game_response.to_owned());
            let should_navigate = match TimeMode::from_str(&game_response.time_mode) {
                Ok(TimeMode::RealTime) => true,
                Ok(TimeMode::Correspondence) | Ok(TimeMode::Untimed) => {
                    let navigation_controller = expect_context::<NavigationControllerSignal>();
                    navigation_controller
                        .signal
                        .get_untracked()
                        .nanoid
                        .is_none()
                }
                _ => false,
            };
            // TODO:
            // use super::refocus::RefocusSignal;
            // let refocus = expect_context::<RefocusSignal>();
            // ... && refocus.signal.get_untracked().focused;
            // this wasn't perfect because then it's easy to miss a game if you tabbed away for a
            // second.
            if should_navigate {
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
        Ok(ServerResult::Ok(ServerMessage::GameSpectate(game))) => {
            let mut games = expect_context::<GamesSignal>();
            games.live_games_add(game);
        }
        Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Created(challenge))))
        | Ok(ServerResult::Ok(ServerMessage::Challenge(ChallengeUpdate::Direct(challenge)))) => {
            let mut challenges = expect_context::<ChallengeStateSignal>();
            challenges.add(vec![challenge]);
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
    game_state.set_game_response(game.clone());
    let mut history = History::new();
    history.moves = game.history.to_owned();
    history.game_type = game.game_type.to_owned();
    if let GameStatus::Finished(result) = &game.game_status {
        history.result = result.to_owned();
    }
    if let Ok(state) = State::new_from_history(&history) {
        game_state.set_state(state, game.black_player.uid, game.white_player.uid);
    }
    // TODO: check if there an answered gc and set it
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
