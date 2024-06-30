use leptos::*;
use shared_types::{ChallengeId, TournamentId};
use std::collections::HashSet;

#[derive(Clone)]
pub struct NotificationContext {
    pub challenges: RwSignal<HashSet<ChallengeId>>,
    pub tournament_invitations: RwSignal<HashSet<TournamentId>>,
    pub tournament_started: RwSignal<HashSet<TournamentId>>,
}

impl NotificationContext {
    pub fn new() -> Self {
        Self {
            challenges: RwSignal::new(HashSet::new()),
            tournament_invitations: RwSignal::new(HashSet::new()),
            tournament_started: RwSignal::new(HashSet::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.challenges.get().is_empty()
            && self.tournament_invitations.get().is_empty()
            && self.tournament_started.get().is_empty()
    }
}

impl Default for NotificationContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_notifications() {
    provide_context(NotificationContext::default())
}
