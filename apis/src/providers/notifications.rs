use leptos::prelude::*;
use shared_types::{ChallengeId, TournamentId};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct NotificationContext {
    pub challenges: RwSignal<HashSet<ChallengeId>>,
    pub tournament_invitations: RwSignal<HashSet<TournamentId>>,
    pub tournament_started: RwSignal<HashSet<TournamentId>>,
    pub tournament_finished: RwSignal<HashSet<TournamentId>>,
    pub schedule_proposals: RwSignal<HashSet<Uuid>>,
    pub schedule_acceptances: RwSignal<HashSet<Uuid>>,
}

impl NotificationContext {
    pub fn new() -> Self {
        Self {
            challenges: RwSignal::new(HashSet::new()),
            tournament_invitations: RwSignal::new(HashSet::new()),
            tournament_started: RwSignal::new(HashSet::new()),
            tournament_finished: RwSignal::new(HashSet::new()),
            schedule_proposals: RwSignal::new(HashSet::new()),
            schedule_acceptances: RwSignal::new(HashSet::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.challenges.with(|v| v.is_empty())
            && self.tournament_invitations.with(|v| v.is_empty())
            && self.tournament_started.with(|v| v.is_empty())
            && self.tournament_finished.with(|v| v.is_empty())
            && self.schedule_proposals.with(|v| v.is_empty())
            && self.schedule_acceptances.with(|v| v.is_empty())
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
