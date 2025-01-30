use crate::responses::{TournamentAbstractResponse, TournamentResponse};
use leptos::prelude::*;
use shared_types::TournamentId;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub struct TournamentStateContext {
    pub full: RwSignal<TournamentState>,
    pub summary: RwSignal<TournamentAbstractState>,
}

impl Default for TournamentStateContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TournamentStateContext {
    pub fn new() -> Self {
        Self {
            full: RwSignal::new(TournamentState::new()),
            summary: RwSignal::new(TournamentAbstractState::new()),
        }
    }

    pub fn remove(&mut self, tournament_id: TournamentId) {
        self.full.update(|s| {
            s.tournaments.remove(&tournament_id);
        });
        self.summary.update(|s| {
            s.tournaments.remove(&tournament_id);
        });
    }

    pub fn add_full(&mut self, tournaments: Vec<TournamentResponse>) {
        self.full.update(|s| {
            for tournament in tournaments {
                s.tournaments
                    .insert(tournament.tournament_id.to_owned(), tournament);
            }
        })
    }

    pub fn add_abstract(&mut self, tournaments: Vec<TournamentAbstractResponse>) {
        self.summary.update(|s| {
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

#[derive(Clone, Debug)]
pub struct TournamentAbstractState {
    pub tournaments: HashMap<TournamentId, TournamentAbstractResponse>,
}

impl TournamentAbstractState {
    pub fn new() -> Self {
        Self {
            tournaments: HashMap::new(),
        }
    }
}

impl Default for TournamentAbstractState {
    fn default() -> Self {
        Self::new()
    }
}
pub fn provide_tournaments() {
    provide_context(TournamentStateContext::new())
}
