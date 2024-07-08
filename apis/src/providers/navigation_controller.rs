use leptos::*;
use shared_types::{GameId, TournamentId};

use crate::{
    common::TournamentAction,
    providers::{api_requests::ApiRequests, chat::Chat, game_state::GameStateSignal},
};

#[derive(Clone, Debug, Copy)]
pub struct NavigationControllerSignal {
    pub game_signal: RwSignal<GameNavigationControllerState>,
    pub tournament_signal: RwSignal<TournamentNavigationControllerState>,
}

impl Default for NavigationControllerSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationControllerSignal {
    pub fn new() -> Self {
        Self {
            game_signal: create_rw_signal(GameNavigationControllerState::new()),
            tournament_signal: create_rw_signal(TournamentNavigationControllerState::new()),
        }
    }

    pub fn update_ids(&mut self, game_id: Option<GameId>, tournament_id: Option<TournamentId>) {
        batch(move || {
            self.game_signal
                .update(|s| game_id.clone_into(&mut s.game_id));
            self.tournament_signal
                .update(|s| tournament_id.clone_into(&mut s.tournament_id));
            if let Some(game_id) = game_id {
                let api = ApiRequests::new();
                let mut game_state = expect_context::<GameStateSignal>();
                let chat = expect_context::<Chat>();
                game_state.set_game_id(game_id.to_owned());
                api.join(game_id);
                chat.typed_message.update(|s| s.clear());
            }
            if let Some(tournament_id) = tournament_id {
                let api = ApiRequests::new();
                let chat = expect_context::<Chat>();
                api.tournament(TournamentAction::Get(tournament_id));
                chat.typed_message.update(|s| s.clear());
            }
        });
    }
}

#[derive(Clone, Debug)]
pub struct TournamentNavigationControllerState {
    pub tournament_id: Option<TournamentId>,
}

impl TournamentNavigationControllerState {
    pub fn new() -> Self {
        Self {
            tournament_id: None,
        }
    }
}

impl Default for TournamentNavigationControllerState {
    fn default() -> Self {
        Self::new()
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
