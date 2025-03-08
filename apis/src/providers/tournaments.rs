use leptos::prelude::*;
use shared_types::TournamentId;

#[derive(Clone, Debug, Copy)]
pub struct TournamentStateContext {
    pub needs_update: RwSignal<Vec<TournamentId>>,
}

impl Default for TournamentStateContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TournamentStateContext {
    pub fn new() -> Self {
        Self {
            needs_update: RwSignal::new(Vec::new()),
        }
    }
    pub fn add_full(&mut self, tournament: TournamentId) {
        self.needs_update.update(|s| {
            s.push(tournament.to_owned());
        });
    }
}

pub fn provide_tournaments() {
    provide_context(TournamentStateContext::new())
}
