use leptos::*;
use shared_types::GameId;
use std::collections::HashMap;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Debug, Copy)]
pub struct TournamentReadySignal {
    pub signal: RwSignal<HashMap<GameId, HashSet<Uuid>>>,
}

impl Default for TournamentReadySignal {
    fn default() -> Self {
        Self::new()
    }
}

impl TournamentReadySignal {
    pub fn new() -> Self {
        Self {
            signal: RwSignal::new(HashMap::new()),
        }
    }
}

pub fn provide_tournament_ready() {
    provide_context(TournamentReadySignal::new())
}
