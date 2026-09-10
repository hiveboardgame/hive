use super::snapshot::apply_snapshot_hash_map;
use crate::responses::ScheduleResponse;
use chrono::{DateTime, Utc};
use leptos::prelude::{
    provide_context,
    RwSignal,
    SetValue,
    StoredValue,
    Update,
    UpdateValue,
    With,
    WithValue,
};
use serde::{Deserialize, Serialize};
use shared_types::{ScheduleOfferStatus, TournamentId};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::tournament_store::TournamentScheduleState;

pub type ScheduleMap = HashMap<Uuid, ScheduleResponse>;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TournamentSchedulesRegistration(u64);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduleLoadKey {
    pub tournament_id: TournamentId,
    pub user_id: Uuid,
    pub load_epoch: u64,
    pub can_view_all_schedules: bool,
    pub(crate) registration: TournamentSchedulesRegistration,
}

#[derive(Clone, Debug)]
struct ActiveTournamentSchedules {
    tournament_id: TournamentId,
    registration: TournamentSchedulesRegistration,
    schedules: TournamentScheduleState,
    viewer: Option<(Uuid, bool)>,
    snapshot_key: Option<ScheduleLoadKey>,
    snapshot_dirty: HashSet<Uuid>,
}

#[derive(Clone, Debug, Copy)]
pub struct SchedulesContext {
    pub notification_schedules: RwSignal<ScheduleMap>,
    notification_resync_dirty: StoredValue<HashSet<Uuid>>,
    tournament: StoredValue<Option<ActiveTournamentSchedules>>,
    next_tournament_registration: StoredValue<u64>,
}

impl SchedulesContext {
    pub fn new() -> Self {
        Self {
            notification_schedules: RwSignal::new(HashMap::new()),
            notification_resync_dirty: StoredValue::new(HashSet::new()),
            tournament: StoredValue::new(None),
            next_tournament_registration: StoredValue::new(0),
        }
    }

    pub(crate) fn set_tournament(
        &self,
        tournament_id: TournamentId,
        schedules: TournamentScheduleState,
    ) -> TournamentSchedulesRegistration {
        let mut registration = TournamentSchedulesRegistration(0);
        self.next_tournament_registration.update_value(|next| {
            *next = next.wrapping_add(1);
            registration = TournamentSchedulesRegistration(*next);
        });
        self.tournament.set_value(Some(ActiveTournamentSchedules {
            tournament_id,
            registration,
            schedules,
            viewer: None,
            snapshot_key: None,
            snapshot_dirty: HashSet::new(),
        }));
        registration
    }

    pub(crate) fn clear_tournament(&self, registration: TournamentSchedulesRegistration) -> bool {
        let mut removed = false;
        self.tournament.update_value(|active| {
            if active
                .as_ref()
                .is_some_and(|active| active.registration == registration)
            {
                *active = None;
                removed = true;
            }
        });
        removed
    }

    pub fn begin_resync(&self) {
        self.notification_resync_dirty.update_value(HashSet::clear);
    }

    pub fn notification_snapshot_apply(&self, snapshot: Vec<ScheduleResponse>) -> Vec<Uuid> {
        self.notification_snapshot_apply_at(snapshot, Utc::now())
    }

    pub(crate) fn notification_snapshot_apply_at(
        &self,
        snapshot: Vec<ScheduleResponse>,
        now: DateTime<Utc>,
    ) -> Vec<Uuid> {
        let dirty = self.notification_resync_dirty.with_value(Clone::clone);
        let snapshot_ids = snapshot.iter().map(|offer| offer.id).collect();
        self.notification_schedules.update(|current| {
            apply_snapshot_hash_map(current, &snapshot_ids, &dirty, snapshot, |offer| offer.id);
        });
        self.notification_resync_dirty.update_value(HashSet::clear);
        self.prune_expired_pending_from_notifications(now)
    }

    pub fn notification_schedule_update(&self, response: &ScheduleResponse) {
        self.mark_notification_dirty([response.id]);
        self.notification_schedules.update(|offers| {
            offers.insert(response.id, response.clone());
        });
    }

    pub fn notification_schedule_delete(&self, offer_id: Uuid) {
        self.mark_notification_dirty([offer_id]);
        self.notification_schedules.update(|offers| {
            offers.remove(&offer_id);
        });
    }

    pub fn notification_ids_for_user(&self, user_id: Uuid) -> (HashSet<Uuid>, HashSet<Uuid>) {
        self.notification_schedules.with(|offers| {
            let mut proposal_ids = HashSet::new();
            let mut acceptance_ids = HashSet::new();
            for offer in offers.values() {
                if offer.status == ScheduleOfferStatus::Pending && offer.opponent_id == user_id {
                    proposal_ids.insert(offer.id);
                } else if offer.status == ScheduleOfferStatus::Accepted
                    && offer.proposer_id == user_id
                    && !offer.notified
                {
                    acceptance_ids.insert(offer.id);
                }
            }
            (proposal_ids, acceptance_ids)
        })
    }

    pub fn begin_tournament_snapshot(&self, key: &ScheduleLoadKey) -> bool {
        let schedules = self.tournament.with_value(|active| {
            active
                .as_ref()
                .filter(|active| {
                    active.tournament_id == key.tournament_id
                        && active.registration == key.registration
                })
                .map(|active| active.schedules)
        });
        let Some(schedules) = schedules else {
            return false;
        };
        self.tournament.update_value(|active| {
            let Some(active) = active.as_mut().filter(|active| {
                active.tournament_id == key.tournament_id && active.registration == key.registration
            }) else {
                return;
            };
            active.viewer = Some((key.user_id, key.can_view_all_schedules));
            active.snapshot_key = Some(key.clone());
            active.snapshot_dirty.clear();
        });
        schedules.update(|offers| {
            offers.retain(|_, offer| {
                visible_to_viewer(offer, key.user_id, key.can_view_all_schedules)
            });
        });
        true
    }

    pub fn tournament_snapshot_apply(
        &self,
        key: &ScheduleLoadKey,
        snapshot: Vec<ScheduleResponse>,
    ) -> bool {
        let Some((schedules, dirty)) = self.tournament.with_value(|active| {
            let active = active.as_ref()?;
            (active.tournament_id == key.tournament_id
                && active.registration == key.registration
                && active.snapshot_key.as_ref() == Some(key))
            .then(|| (active.schedules, active.snapshot_dirty.clone()))
        }) else {
            return false;
        };
        let snapshot = snapshot
            .into_iter()
            .filter(|offer| {
                offer.tournament_id == key.tournament_id
                    && visible_to_viewer(offer, key.user_id, key.can_view_all_schedules)
            })
            .collect::<Vec<_>>();
        let snapshot_ids = snapshot.iter().map(|offer| offer.id).collect();
        schedules.update(|current| {
            apply_snapshot_hash_map(current, &snapshot_ids, &dirty, snapshot, |offer| offer.id);
            current.retain(|_, offer| {
                visible_to_viewer(offer, key.user_id, key.can_view_all_schedules)
            });
        });
        self.finish_tournament_snapshot(key);
        true
    }

    pub fn finish_tournament_snapshot(&self, key: &ScheduleLoadKey) -> bool {
        let mut finished = false;
        self.tournament.update_value(|active| {
            let Some(active) = active.as_mut().filter(|active| {
                active.tournament_id == key.tournament_id
                    && active.registration == key.registration
                    && active.snapshot_key.as_ref() == Some(key)
            }) else {
                return;
            };
            active.snapshot_key = None;
            active.snapshot_dirty.clear();
            finished = true;
        });
        finished
    }

    pub(crate) fn clear_tournament_schedules(
        &self,
        tournament_id: &TournamentId,
        registration: TournamentSchedulesRegistration,
    ) -> bool {
        let schedules = self.tournament.with_value(|active| {
            active
                .as_ref()
                .filter(|active| {
                    &active.tournament_id == tournament_id && active.registration == registration
                })
                .map(|active| active.schedules)
        });
        let Some(schedules) = schedules else {
            return false;
        };
        self.tournament.update_value(|active| {
            let Some(active) = active.as_mut().filter(|active| {
                &active.tournament_id == tournament_id && active.registration == registration
            }) else {
                return;
            };
            active.viewer = None;
            active.snapshot_key = None;
            active.snapshot_dirty.clear();
        });
        schedules.update(ScheduleMap::clear);
        true
    }

    fn clear_tournament_schedules_by_id(&self, tournament_id: &TournamentId) {
        let schedules = self.tournament.with_value(|active| {
            active
                .as_ref()
                .filter(|active| &active.tournament_id == tournament_id)
                .map(|active| active.schedules)
        });
        self.tournament.update_value(|active| {
            let Some(active) = active
                .as_mut()
                .filter(|active| &active.tournament_id == tournament_id)
            else {
                return;
            };
            active.viewer = None;
            active.snapshot_key = None;
            active.snapshot_dirty.clear();
        });
        if let Some(schedules) = schedules {
            schedules.update(ScheduleMap::clear);
        }
    }

    pub fn schedule_update(&self, response: &ScheduleResponse) {
        self.mark_tournament_dirty([response.id]);
        self.with_tournament_map(response, |offers, user_id, can_view_all| {
            if visible_to_viewer(response, user_id, can_view_all) {
                offers.insert(response.id, response.clone());
            } else {
                offers.remove(&response.id);
            }
        });
    }

    pub fn purge_tournament(&self, tournament_id: &TournamentId) {
        let notification_ids = self.notification_schedules.with(|offers| {
            offers
                .values()
                .filter(|offer| &offer.tournament_id == tournament_id)
                .map(|offer| offer.id)
                .collect::<Vec<_>>()
        });
        self.mark_notification_dirty(notification_ids.iter().copied());
        self.notification_schedules.update(|offers| {
            offers.retain(|_, offer| &offer.tournament_id != tournament_id);
        });
        self.clear_tournament_schedules_by_id(tournament_id);
    }

    pub fn prune_expired_pending(&self, now: DateTime<Utc>) -> Vec<Uuid> {
        self.prune_expired_pending_from_notifications(now)
    }

    fn prune_expired_pending_from_notifications(&self, now: DateTime<Utc>) -> Vec<Uuid> {
        let expired = self.notification_schedules.with(|offers| {
            offers
                .values()
                .filter(|offer| {
                    offer.status == ScheduleOfferStatus::Pending && !offer.has_future_candidate(now)
                })
                .map(|offer| offer.id)
                .collect::<Vec<_>>()
        });
        if !expired.is_empty() {
            self.mark_notification_dirty(expired.iter().copied());
            self.notification_schedules.update(|offers| {
                for offer_id in &expired {
                    offers.remove(offer_id);
                }
            });
        }
        expired
    }

    fn with_tournament_map(
        &self,
        response: &ScheduleResponse,
        mutate: impl FnOnce(&mut ScheduleMap, Uuid, bool),
    ) {
        let active = self.tournament.with_value(|active| {
            let active = active
                .as_ref()
                .filter(|active| active.tournament_id == response.tournament_id)?;
            let (user_id, can_view_all) = active.viewer?;
            Some((active.schedules, user_id, can_view_all))
        });
        if let Some((schedules, user_id, can_view_all)) = active {
            schedules.update(|offers| mutate(offers, user_id, can_view_all));
        }
    }

    fn mark_notification_dirty(&self, offer_ids: impl IntoIterator<Item = Uuid>) {
        self.notification_resync_dirty.update_value(|dirty| {
            dirty.extend(offer_ids);
        });
    }

    fn mark_tournament_dirty(&self, offer_ids: impl IntoIterator<Item = Uuid>) {
        self.tournament.update_value(|active| {
            let Some(active) = active
                .as_mut()
                .filter(|active| active.snapshot_key.is_some())
            else {
                return;
            };
            active.snapshot_dirty.extend(offer_ids);
        });
    }
}

fn visible_to_viewer(offer: &ScheduleResponse, user_id: Uuid, can_view_all: bool) -> bool {
    can_view_all || offer.proposer_id == user_id || offer.opponent_id == user_id
}

impl Default for SchedulesContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_schedules() {
    provide_context(SchedulesContext::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use leptos::prelude::Owner;
    use reactive_stores::Store;

    fn schedule_state() -> TournamentScheduleState {
        Store::new(HashMap::new())
    }

    fn offer(
        id: u128,
        tournament_id: &TournamentId,
        proposer_id: Uuid,
        opponent_id: Uuid,
        status: ScheduleOfferStatus,
        candidate: DateTime<Utc>,
    ) -> ScheduleResponse {
        let resolved = status != ScheduleOfferStatus::Pending;
        ScheduleResponse {
            id: Uuid::from_u128(id),
            slot_id: Uuid::from_u128(7),
            slot_context: String::from("Game 1"),
            tournament_name: String::from("Cup"),
            tournament_id: tournament_id.clone(),
            proposer_id,
            proposer_username: String::from("alice"),
            opponent_id,
            opponent_username: String::from("bob"),
            candidate_times: vec![candidate],
            status,
            selected_time: (status == ScheduleOfferStatus::Accepted).then_some(candidate),
            created_at: candidate - Duration::hours(2),
            resolved_at: resolved.then_some(candidate - Duration::hours(1)),
            resolved_by: resolved.then_some(opponent_id),
            notified: false,
        }
    }

    fn load_key(
        tournament_id: TournamentId,
        user_id: Uuid,
        registration: TournamentSchedulesRegistration,
    ) -> ScheduleLoadKey {
        ScheduleLoadKey {
            tournament_id,
            user_id,
            load_epoch: 1,
            can_view_all_schedules: false,
            registration,
        }
    }

    #[test]
    fn live_offer_update_survives_stale_tournament_snapshot() {
        let owner = Owner::new();
        owner.with(|| {
            let tournament_id = TournamentId(String::from("cup"));
            let proposer_id = Uuid::from_u128(11);
            let opponent_id = Uuid::from_u128(12);
            let candidate = DateTime::from_timestamp(4_000_000_000, 0).unwrap();
            let stale = offer(
                1,
                &tournament_id,
                proposer_id,
                opponent_id,
                ScheduleOfferStatus::Pending,
                candidate,
            );
            let live = offer(
                1,
                &tournament_id,
                proposer_id,
                opponent_id,
                ScheduleOfferStatus::Accepted,
                candidate,
            );
            let schedules = schedule_state();
            let context = SchedulesContext::new();
            let registration = context.set_tournament(tournament_id.clone(), schedules);
            let key = load_key(tournament_id, proposer_id, registration);
            assert!(context.begin_tournament_snapshot(&key));

            context.schedule_update(&live);
            assert!(context.tournament_snapshot_apply(&key, vec![stale]));

            schedules.with(|offers| assert_eq!(offers.get(&live.id), Some(&live)));
        });
    }

    #[test]
    fn same_id_replacement_rejects_stale_cleanup_and_snapshot() {
        let owner = Owner::new();
        owner.with(|| {
            let tournament_id = TournamentId(String::from("cup"));
            let proposer_id = Uuid::from_u128(31);
            let opponent_id = Uuid::from_u128(32);
            let candidate = DateTime::from_timestamp(4_000_000_000, 0).unwrap();
            let stale_offer = offer(
                4,
                &tournament_id,
                proposer_id,
                opponent_id,
                ScheduleOfferStatus::Pending,
                candidate,
            );
            let current_offer = offer(
                5,
                &tournament_id,
                proposer_id,
                opponent_id,
                ScheduleOfferStatus::Pending,
                candidate,
            );
            let context = SchedulesContext::new();
            let old_schedules = schedule_state();
            let old_registration = context.set_tournament(tournament_id.clone(), old_schedules);
            let old_key = load_key(tournament_id.clone(), proposer_id, old_registration);
            assert!(context.begin_tournament_snapshot(&old_key));

            let current_schedules = schedule_state();
            let current_registration =
                context.set_tournament(tournament_id.clone(), current_schedules);
            let current_key = load_key(tournament_id.clone(), proposer_id, current_registration);
            assert!(context.begin_tournament_snapshot(&current_key));

            assert!(!context.clear_tournament(old_registration));
            assert!(!context.tournament_snapshot_apply(&old_key, vec![stale_offer]));
            assert!(!context.finish_tournament_snapshot(&old_key));
            assert!(!context.clear_tournament_schedules(&tournament_id, old_registration,));
            current_schedules.with(|offers| assert!(offers.is_empty()));

            assert!(context.tournament_snapshot_apply(&current_key, vec![current_offer.clone()]));
            current_schedules
                .with(|offers| assert_eq!(offers.get(&current_offer.id), Some(&current_offer)));
            assert!(context.clear_tournament(current_registration));
        });
    }

    #[test]
    fn live_notification_update_and_delete_survive_stale_snapshot() {
        let owner = Owner::new();
        owner.with(|| {
            let tournament_id = TournamentId(String::from("cup"));
            let proposer_id = Uuid::from_u128(21);
            let opponent_id = Uuid::from_u128(22);
            let candidate = DateTime::from_timestamp(4_000_000_000, 0).unwrap();
            let stale_update = offer(
                2,
                &tournament_id,
                proposer_id,
                opponent_id,
                ScheduleOfferStatus::Pending,
                candidate,
            );
            let live_update = offer(
                2,
                &tournament_id,
                proposer_id,
                opponent_id,
                ScheduleOfferStatus::Accepted,
                candidate,
            );
            let deleted = offer(
                3,
                &tournament_id,
                opponent_id,
                proposer_id,
                ScheduleOfferStatus::Pending,
                candidate,
            );
            let context = SchedulesContext::new();
            context.notification_schedule_update(&deleted);
            context.begin_resync();

            context.notification_schedule_update(&live_update);
            context.notification_schedule_delete(deleted.id);
            context.notification_snapshot_apply_at(
                vec![stale_update, deleted.clone()],
                candidate - Duration::days(1),
            );

            context.notification_schedules.with(|offers| {
                assert_eq!(offers.get(&live_update.id), Some(&live_update));
                assert!(!offers.contains_key(&deleted.id));
            });
        });
    }
}
