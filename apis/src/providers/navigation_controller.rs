use lazy_static::lazy_static;
use leptos::*;
use regex::Regex;

use crate::providers::{api_requests::ApiRequests, game_state::GameStateSignal};

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

    pub fn update_nanoid(&mut self, nanoid: Option<String>) {
        self.signal.update(|s| s.nanoid = nanoid.to_owned());
        let api = ApiRequests::new();
        if let Some(game_id) = nanoid {
            let mut game_state = expect_context::<GameStateSignal>();
            game_state.set_game_id(game_id.to_owned());
            api.join(game_id.to_owned());
        }
    }
}

#[derive(Clone, Debug)]
pub struct NavigationControllerState {
    pub nanoid: Option<String>,
}

impl NavigationControllerState {
    pub fn new() -> Self {
        Self { nanoid: None }
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
