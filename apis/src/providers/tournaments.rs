use std::collections::HashSet;
use leptos::prelude::*;
use shared_types::TournamentId;

#[derive(Clone, Debug, Copy, Default)]
pub struct TournamentStateContext {
    pub needs_update: RwSignal<HashSet<TournamentId>>,
}

impl TournamentStateContext {
    pub fn add_full(&mut self, tournament: TournamentId) {
        self.needs_update.update(|s| {
            s.insert(tournament.to_owned());
        });
    }
}

pub fn provide_tournaments() {
    provide_context(TournamentStateContext::default())
}
