use leptos::*;
use shared_types::{ApisId, ChallengeId, TournamentId};
use std::collections::HashSet;

#[derive(Clone)]
pub struct NotificationContext {
    pub challenges: RwSignal<HashSet<ChallengeId>>,
    pub tournament_invitations: RwSignal<HashSet<TournamentId>>,
}

impl NotificationContext {
    pub fn new() -> Self {
        Self {
            challenges: RwSignal::new(HashSet::new()),
            tournament_invitations: RwSignal::new(HashSet::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.challenges.get().is_empty() && self.tournament_invitations.get().is_empty()
    }

    pub fn remove(&mut self, notification: &ApisId) {
        match notification {
            ApisId::Challenge(challenge_id) => {
                self.challenges.update(|s| {
                    s.remove(challenge_id);
                });
            }
            ApisId::Tournament(tournament_id) => {
                self.tournament_invitations.update(|s| {
                    s.remove(tournament_id);
                });
            }
            _ => unimplemented!(),
        }
    }

    pub fn add(&mut self, notifications: Vec<ApisId>) {
        for notification in notifications {
            match notification {
                ApisId::Challenge(challenge_id) => {
                    self.challenges.update(|s| {
                        s.insert(challenge_id);
                    });
                }
                ApisId::Tournament(tournament_id) => {
                    self.tournament_invitations.update(|s| {
                        s.insert(tournament_id);
                    });
                }
                _ => unimplemented!(),
            }
        }
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
