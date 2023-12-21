use crate::providers::api_requests::ApiRequests;
use leptos::{create_rw_signal, provide_context, RwSignal, SignalUpdate};

#[derive(Clone, Debug, Copy)]
pub struct GameControllerSignal {
    pub signal: RwSignal<GameController>,
}

impl Default for GameControllerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl GameControllerSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(GameController::new()),
        }
    }

    pub fn join(&mut self, nanoid: String, username: Option<String>) {
        self.signal.update(|s| s.join(nanoid, username));
    }

    pub fn set_next_games(&mut self, games: Vec<String>) {
        self.signal.update(|s| s.next_games = games)
    }
}

#[derive(Clone, Debug)]
pub struct GameController {
    pub username: Option<String>,
    // game_id is the nanoid of the game
    pub current_game_id: Option<String>,
    // games that need user input
    pub next_games: Vec<String>,
}

impl Default for GameController {
    fn default() -> Self {
        Self::new()
    }
}

impl GameController {
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

pub fn provide_game_controller() {
    provide_context(GameControllerSignal::new())
}
