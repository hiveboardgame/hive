use leptos::*;

use crate::{
    common::TournamentAction,
    providers::{api_requests::ApiRequests, chat::Chat, game_state::GameStateSignal},
};

#[derive(Clone, Debug, Copy)]
pub struct NavigationControllerSignal {
    pub game_signal: RwSignal<NavigationControllerState>,
    pub tournament_signal: RwSignal<NavigationControllerState>,
}

impl Default for NavigationControllerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationControllerSignal {
    pub fn new() -> Self {
        Self {
            game_signal: create_rw_signal(NavigationControllerState::new()),
            tournament_signal: create_rw_signal(NavigationControllerState::new()),
        }
    }

    pub fn update_nanoids(
        &mut self,
        game_nanoid: Option<String>,
        tournament_nanoid: Option<String>,
    ) {
        batch(move || {
            self.game_signal
                .update(|s| game_nanoid.clone_into(&mut s.nanoid));
            self.tournament_signal
                .update(|s| tournament_nanoid.clone_into(&mut s.nanoid));
            if let Some(game_id) = game_nanoid {
                let api = ApiRequests::new();
                let mut game_state = expect_context::<GameStateSignal>();
                let chat = expect_context::<Chat>();
                game_state.set_game_id(game_id.to_owned());
                api.join(game_id.to_owned());
                chat.typed_message.update(|s| s.clear());
            }
            if let Some(tournament_id) = tournament_nanoid {
                let api = ApiRequests::new();
                api.tournament(TournamentAction::Get(tournament_id))
            }
        });
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
