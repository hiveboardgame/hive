use crate::responses::TournamentResponse;
use leptos::*;
use shared_types::TournamentId;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub struct TournamentStateSignal {
    pub signal: RwSignal<TournamentState>,
}

impl Default for TournamentStateSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl TournamentStateSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(TournamentState::new()),
        }
    }

    pub fn remove(&mut self, tournament_id: TournamentId) {
        self.signal.update(|s| {
            s.tournaments.remove(&tournament_id);
        });
    }

    pub fn add(&mut self, tournaments: Vec<TournamentResponse>) {
        self.signal.update(|s| {
            for tournament in tournaments {
                s.tournaments
                    .insert(tournament.tournament_id.to_owned(), tournament);
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct TournamentState {
    pub tournaments: HashMap<TournamentId, TournamentResponse>,
}

impl TournamentState {
    pub fn new() -> Self {
        Self {
            tournaments: HashMap::new(),
        }
    }
}

impl Default for TournamentState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_tournaments() {
    provide_context(TournamentStateSignal::new())
}
