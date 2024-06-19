use lazy_static::lazy_static;
use leptos::*;
use regex::Regex;
use shared_types::GameId;

use crate::providers::{api_requests::ApiRequests, chat::Chat, game_state::GameStateSignal};

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[derive(Clone, Debug, Copy)]
pub struct NavigationControllerSignal {
    pub signal: RwSignal<NavigationControllerState>,
}

impl Default for NavigationControllerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationControllerSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(NavigationControllerState::new()),
        }
    }

    pub fn update_game_id(&mut self, game_id: Option<GameId>) {
        batch(move || {
            self.signal.update(|s| game_id.clone_into(&mut s.game_id));
            let api = ApiRequests::new();
            if let Some(game_id) = game_id {
                let mut game_state = expect_context::<GameStateSignal>();
                let chat = expect_context::<Chat>();
                game_state.set_game_id(game_id.to_owned());
                api.join(game_id);
                chat.typed_message.update(|s| s.clear());
            }
        });
    }
}

#[derive(Clone, Debug)]
pub struct NavigationControllerState {
    pub game_id: Option<GameId>,
}

impl NavigationControllerState {
    pub fn new() -> Self {
        Self { game_id: None }
    }
}

impl Default for NavigationControllerState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_navigation_controller() {
    provide_context(NavigationControllerSignal::new())
}
