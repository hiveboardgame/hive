use crate::providers::{chat::Chat, game_state::GameStateSignal};
use leptos::prelude::*;
use shared_types::GameId;

use super::ApiRequestsProvider;

#[derive(Clone, Debug, Copy)]
pub struct NavigationControllerSignal {
    pub game_signal: RwSignal<GameNavigationControllerState>,
    pub redirect: RwSignal<String>,
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
            redirect: RwSignal::new("/".to_owned()),
        }
    }

    pub fn update_id(&mut self, game_id: Option<GameId>) {
        let api = expect_context::<ApiRequestsProvider>().0.get();
        let mut game_state = expect_context::<GameStateSignal>();
        let chat = expect_context::<Chat>();

        self.game_signal
            .update(|s| game_id.clone_into(&mut s.game_id));
        if let Some(game_id) = game_id {
            game_state.set_game_id(game_id.to_owned());
            api.join(game_id);
            chat.typed_message.update(|s| s.clear());
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
