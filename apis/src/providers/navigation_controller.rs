use leptos::prelude::*;
use shared_types::GameId;

#[derive(Clone, Debug, Copy)]
pub struct NavigationControllerSignal {
    pub game_signal: RwSignal<GameNavigationControllerState>,
}

impl Default for NavigationControllerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationControllerSignal {
    pub fn new() -> Self {
        Self {
            game_signal: RwSignal::new(GameNavigationControllerState::new()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameNavigationControllerState {
    pub game_id: Option<GameId>,
}

impl GameNavigationControllerState {
    pub fn new() -> Self {
        Self { game_id: None }
    }
}

impl Default for GameNavigationControllerState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_navigation_controller() {
    provide_context(NavigationControllerSignal::new())
}
