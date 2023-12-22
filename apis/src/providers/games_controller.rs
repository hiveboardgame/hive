use crate::providers::api_requests::ApiRequests;
use lazy_static::lazy_static;
use leptos::logging::log;
use leptos::*;
use leptos::{create_rw_signal, provide_context, RwSignal, SignalUpdate};
use leptos_router::RouterContext;
use regex::Regex;

use super::auth_context::AuthContext;
use super::web_socket::WebsocketContext;

#[derive(Clone, Debug, Copy)]
pub struct GamesControllerSignal {
    pub signal: RwSignal<GamesController>,
}

impl Default for GamesControllerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesControllerSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(GamesController::new()),
        }
    }

    pub fn join(&mut self, username: Option<String>, nanoid: String) {
        log!("GamesController is joining");
        self.signal.get().join(username, nanoid);
    }

    pub fn set_next_games(&mut self, games: Vec<String>) {
        self.signal.update(|s| s.next_games = games)
    }
}

#[derive(Clone, Debug)]
pub struct GamesController {
    pub username: Option<String>,
    // game_id is the nanoid of the game
    pub current_game_id: Option<String>,
    // games that need user input
    pub next_games: Vec<String>,
    pub websocket_connection_established: bool,
}

impl Default for GamesController {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

impl GamesController {
    // TODO get the state from URL/game_id via a call
    pub fn new() -> Self {
        Self {
            username: None,
            current_game_id: None,
            next_games: vec![],
            websocket_connection_established: false,
        }
    }

    pub fn current_page_game_id() -> Option<String> {
        let router = expect_context::<RouterContext>();
        if let Some(caps) = NANOID.captures(&(router.pathname().get_untracked())) {
            if let Some(m) = caps.name("nanoid") {
                return Some(m.as_str().to_owned());
            }
        }
        None
    }

    pub fn username(&self) -> Option<String> {
        let auth_context = expect_context::<AuthContext>();
        let account_response = (auth_context.user)();
        if let Some(Ok(Some(user))) = account_response {
            return Some(user.username);
        }
        None
    }

    pub fn join(&mut self, username: Option<String>, nanoid: String) {
        if self.current_game_id == Some(nanoid.to_owned()) && username == self.username {
            return;
        }
        if let Some(_) = use_context::<WebsocketContext>() {
            log!("we actually got here");
            ApiRequests::new().join(nanoid.to_owned());
            self.current_game_id = Some(nanoid);
            self.username = username;
        }
    }
}

pub fn provide_games_controller() {
    provide_context(GamesControllerSignal::new())
}
