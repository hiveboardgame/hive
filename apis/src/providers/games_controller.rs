use crate::providers::api_requests::ApiRequests;
use leptos::{create_rw_signal, provide_context, RwSignal, SignalUpdate};

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

    pub fn join(&mut self, nanoid: String, username: Option<String>) {
        // join can just get all the info itself and doesn't need to be mut
        self.signal.update(|s| s.join(nanoid, username));
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
}

impl Default for GamesController {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesController {
    // TODO get the state from URL/game_id via a call
    pub fn new() -> Self {
        Self {
            username: None,
            current_game_id: None,
            next_games: vec![],
        }
    }

    pub fn join(&mut self, nanoid: String, username: Option<String>) {
        if self.current_game_id == Some(nanoid.to_owned()) && username == self.username {
            return;
        }
        ApiRequests::new().join(nanoid.to_owned());
        self.current_game_id = Some(nanoid);
    }
}

pub fn provide_games_controller() {
    provide_context(GamesControllerSignal::new())
}
