use super::snapshot::apply_snapshot_set;
use leptos::prelude::*;
use shared_types::{ChallengeId, TournamentId};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct NotificationContext {
    pub challenges: RwSignal<HashSet<ChallengeId>>,
    tournament_invitations: RwSignal<HashSet<TournamentId>>,
    pub tournament_started: RwSignal<HashSet<TournamentId>>,
    pub tournament_finished: RwSignal<HashSet<TournamentId>>,
    schedule_proposals: RwSignal<HashSet<Uuid>>,
    schedule_acceptances: RwSignal<HashSet<Uuid>>,
    tournament_invitation_resync_dirty: StoredValue<HashSet<TournamentId>>,
    schedule_notification_resync_dirty: StoredValue<HashSet<Uuid>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleNotificationKind {
    Proposal,
    Acceptance,
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
            tournament_invitation_resync_dirty: StoredValue::new(HashSet::new()),
            schedule_notification_resync_dirty: StoredValue::new(HashSet::new()),
        }
    }

    pub fn begin_resync(&self) {
        self.tournament_invitation_resync_dirty
            .update_value(|d| d.clear());
        self.schedule_notification_resync_dirty
            .update_value(|d| d.clear());
    }

    pub fn is_empty(&self) -> bool {
        self.challenges.with(|v| v.is_empty())
            && self.tournament_invitations.with(|v| v.is_empty())
            && self.tournament_started.with(|v| v.is_empty())
            && self.tournament_finished.with(|v| v.is_empty())
            && self.schedule_proposals.with(|v| v.is_empty())
            && self.schedule_acceptances.with(|v| v.is_empty())
    }

    pub fn has_tournament_notifications(&self) -> bool {
        !self.tournament_invitations.with(|v| v.is_empty())
            || !self.tournament_started.with(|v| v.is_empty())
            || !self.tournament_finished.with(|v| v.is_empty())
    }

    pub fn sorted_tournament_notification_ids(&self) -> Vec<TournamentId> {
        let mut tournament_ids = self.tournament_invitations();
        tournament_ids.extend(self.tournament_started.get());
        tournament_ids.extend(self.tournament_finished.get());
        let mut tournament_ids = tournament_ids.into_iter().collect::<Vec<_>>();
        tournament_ids.sort_by(|a, b| a.0.cmp(&b.0));
        tournament_ids
    }

    pub fn tournament_invitations(&self) -> HashSet<TournamentId> {
        self.tournament_invitations.get()
    }

    pub fn schedule_proposals(&self) -> HashSet<Uuid> {
        self.schedule_proposals.get()
    }

    pub fn schedule_acceptances(&self) -> HashSet<Uuid> {
        self.schedule_acceptances.get()
    }

    pub fn tournament_invitation_insert(&self, tournament_id: TournamentId) {
        self.mark_tournament_invitation_dirty(&tournament_id);
        self.tournament_invitations.update(|invitations| {
            invitations.insert(tournament_id);
        });
    }

    pub fn tournament_invitation_remove(&self, tournament_id: &TournamentId) {
        self.mark_tournament_invitation_dirty(tournament_id);
        self.tournament_invitations.update(|invitations| {
            invitations.remove(tournament_id);
        });
    }

    pub fn tournament_invitations_snapshot_apply(&self, invitations: Vec<TournamentId>) {
        let dirty: HashSet<TournamentId> = self
            .tournament_invitation_resync_dirty
            .with_value(|d| d.clone());
        let snapshot_ids: HashSet<TournamentId> = invitations.into_iter().collect();
        self.tournament_invitations.update(|current| {
            apply_snapshot_set(current, &snapshot_ids, &dirty);
        });
        self.tournament_invitation_resync_dirty
            .update_value(|d| d.clear());
    }

    pub fn schedule_notification_insert(&self, kind: ScheduleNotificationKind, schedule_id: Uuid) {
        self.mark_schedule_notification_dirty(schedule_id);
        match kind {
            ScheduleNotificationKind::Proposal => {
                self.schedule_proposals.update(|proposals| {
                    proposals.insert(schedule_id);
                });
            }
            ScheduleNotificationKind::Acceptance => {
                self.schedule_acceptances.update(|acceptances| {
                    acceptances.insert(schedule_id);
                });
            }
        }
    }

    pub fn schedule_notification_remove(&self, schedule_id: Uuid) {
        self.mark_schedule_notification_dirty(schedule_id);
        self.schedule_proposals.update(|proposals| {
            proposals.remove(&schedule_id);
        });
        self.schedule_acceptances.update(|acceptances| {
            acceptances.remove(&schedule_id);
        });
    }

    pub fn schedule_notifications_snapshot_apply(
        &self,
        proposal_ids: HashSet<Uuid>,
        acceptance_ids: HashSet<Uuid>,
    ) {
        let dirty_ids: HashSet<Uuid> = self
            .schedule_notification_resync_dirty
            .with_value(|d| d.clone());

        self.schedule_proposals.update(|current| {
            apply_snapshot_set(current, &proposal_ids, &dirty_ids);
        });
        self.schedule_acceptances.update(|current| {
            apply_snapshot_set(current, &acceptance_ids, &dirty_ids);
        });
        self.schedule_notification_resync_dirty
            .update_value(|d| d.clear());
    }

    fn mark_tournament_invitation_dirty(&self, tournament_id: &TournamentId) {
        self.tournament_invitation_resync_dirty.update_value(|d| {
            d.insert(tournament_id.clone());
        });
    }

    fn mark_schedule_notification_dirty(&self, schedule_id: Uuid) {
        self.schedule_notification_resync_dirty.update_value(|d| {
            d.insert(schedule_id);
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tournament_id(id: &str) -> TournamentId {
        TournamentId(id.to_string())
    }

    fn uuid_set(ids: impl IntoIterator<Item = Uuid>) -> HashSet<Uuid> {
        ids.into_iter().collect()
    }

    #[test]
    fn tournament_invitation_snapshot_drops_absent_ids() {
        let owner = Owner::new();
        owner.with(|| {
            let notifications = NotificationContext::new();
            notifications.tournament_invitation_insert(tournament_id("old"));
            notifications.begin_resync();

            notifications.tournament_invitations_snapshot_apply(vec![tournament_id("new")]);

            let ids = notifications.tournament_invitations();
            assert!(!ids.contains(&tournament_id("old")));
            assert!(ids.contains(&tournament_id("new")));
        });
    }

    #[test]
    fn tournament_invitation_snapshot_preserves_dirty_add() {
        let owner = Owner::new();
        owner.with(|| {
            let notifications = NotificationContext::new();
            notifications.begin_resync();
            notifications.tournament_invitation_insert(tournament_id("late"));

            notifications.tournament_invitations_snapshot_apply(vec![tournament_id("early")]);

            let ids = notifications.tournament_invitations();
            assert!(ids.contains(&tournament_id("late")));
            assert!(ids.contains(&tournament_id("early")));
        });
    }

    #[test]
    fn tournament_invitation_snapshot_preserves_dirty_remove() {
        let owner = Owner::new();
        owner.with(|| {
            let notifications = NotificationContext::new();
            let doomed = tournament_id("doomed");
            notifications.tournament_invitation_insert(doomed.clone());
            notifications.begin_resync();
            notifications.tournament_invitation_remove(&doomed);

            notifications.tournament_invitations_snapshot_apply(vec![doomed.clone()]);

            assert!(!notifications.tournament_invitations().contains(&doomed));
        });
    }

    #[test]
    fn schedule_notification_snapshot_drops_absent_ids() {
        let owner = Owner::new();
        owner.with(|| {
            let notifications = NotificationContext::new();
            let old = Uuid::new_v4();
            let new = Uuid::new_v4();
            notifications.schedule_notification_insert(ScheduleNotificationKind::Proposal, old);
            notifications.begin_resync();

            notifications.schedule_notifications_snapshot_apply(uuid_set([new]), HashSet::new());

            let proposals = notifications.schedule_proposals();
            assert!(!proposals.contains(&old));
            assert!(proposals.contains(&new));
        });
    }

    #[test]
    fn schedule_notification_snapshot_preserves_dirty_add_and_remove() {
        let owner = Owner::new();
        owner.with(|| {
            let notifications = NotificationContext::new();
            let removed = Uuid::new_v4();
            let added = Uuid::new_v4();
            notifications
                .schedule_notification_insert(ScheduleNotificationKind::Acceptance, removed);
            notifications.begin_resync();
            notifications.schedule_notification_remove(removed);
            notifications.schedule_notification_insert(ScheduleNotificationKind::Proposal, added);

            notifications
                .schedule_notifications_snapshot_apply(HashSet::new(), uuid_set([removed]));

            assert!(!notifications.schedule_acceptances().contains(&removed));
            assert!(notifications.schedule_proposals().contains(&added));
        });
    }

    #[test]
    fn schedule_notification_snapshot_updates_notifications() {
        let owner = Owner::new();
        owner.with(|| {
            let notifications = NotificationContext::new();
            let old = Uuid::new_v4();
            let new = Uuid::new_v4();
            notifications.schedule_notification_insert(ScheduleNotificationKind::Proposal, old);
            notifications.begin_resync();

            notifications.schedule_notifications_snapshot_apply(uuid_set([new]), HashSet::new());

            assert!(!notifications.schedule_proposals().contains(&old));
            assert_eq!(notifications.schedule_proposals(), uuid_set([new]));
        });
    }
}
